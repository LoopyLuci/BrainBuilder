//! OS-level process containment, layered on top of the portable
//! capability/timeout checks in `supervisor.rs`. On Windows: a real Job
//! Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (dropping the handle
//! kills the whole process tree — covers grandchildren a plain
//! `Child::kill()` misses) and an optional process memory ceiling. On
//! Unix: a real `RLIMIT_AS` ceiling applied in the child before it execs
//! (`harden_before_spawn`, called by `Supervisor` ahead of `spawn()` — a
//! Job Object's limits can be set after spawn, but rlimits can't), plus a
//! private process group so `kill_process_tree` can `killpg` every
//! descendant instead of only the immediate child. **Written against
//! documented `libc`/`std::os::unix::process` APIs but not run on a real
//! Linux/macOS machine** — this repo's only development environment is
//! Windows. The GitHub Actions Linux/macOS runners (`CI/CD` job) are what
//! should first prove this path out; treat it as real code, not yet as a
//! verified guarantee. cgroups/seccomp/App-Sandbox hardening beyond a
//! memory rlimit and a group kill remains a real follow-up.

#[cfg(windows)]
mod imp {
    use std::os::windows::io::AsRawHandle;
    use std::process::{Child, Command};
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    pub struct JobObject(*mut core::ffi::c_void);

    // A Windows HANDLE is an opaque, process-wide identifier — safe to use
    // from any thread, unlike a general raw pointer (which is why it isn't
    // Send/Sync by default here).
    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}

    impl JobObject {
        /// Creates a job with kill-on-close set, and an optional total
        /// memory ceiling in bytes. Returns `None` if the OS call fails —
        /// callers treat that as "no hardening available", not a hard error,
        /// since the portable capability/timeout checks still apply.
        pub fn new(memory_limit_bytes: Option<u64>) -> Option<Self> {
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return None;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if let Some(mem) = memory_limit_bytes {
                    info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
                    info.JobMemoryLimit = mem as usize;
                }
                let ok = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    std::ptr::addr_of!(info).cast(),
                    std::mem::size_of_val(&info) as u32,
                );
                if ok == 0 {
                    CloseHandle(job);
                    return None;
                }
                Some(Self(job))
            }
        }

        /// Assigns `child` to this job. Best-effort: returns whether it
        /// succeeded, doesn't error the caller's spawn if it didn't.
        pub fn assign(&self, child: &Child) -> bool {
            unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle().cast()) != 0 }
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    /// Nothing to do pre-spawn on Windows — the memory ceiling and
    /// kill-on-close tree containment are both applied post-spawn via
    /// `JobObject::new`/`assign` (see `Supervisor::run_checked_named`).
    pub fn harden_before_spawn(_command: &mut Command, _memory_limit_bytes: Option<u64>) {}

    /// A direct kill of the immediate child. Grandchildren are reaped by the
    /// assigned Job Object's kill-on-close semantics once `_job` (held by
    /// the caller) drops — this call handles the process that's actually
    /// hung, which the Job Object teardown alone doesn't immediately signal.
    pub fn kill_process_tree(child: &mut Child) {
        let _ = child.kill();
    }
}

#[cfg(not(windows))]
mod imp {
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};

    pub struct JobObject;

    impl JobObject {
        /// Unix hardening (rlimit + process group) must be applied *before*
        /// `spawn()` — see `harden_before_spawn`, called by `Supervisor`
        /// ahead of it. There is nothing left to attach post-spawn, unlike
        /// the Windows Job Object.
        pub fn new(_memory_limit_bytes: Option<u64>) -> Option<Self> {
            None
        }
        pub fn assign(&self, _child: &Child) -> bool {
            false
        }
    }

    /// Puts the about-to-be-spawned child in its own process group (so its
    /// pgid equals its pid, letting `kill_process_tree` reach every
    /// descendant via `killpg`) and, if given, installs a real `RLIMIT_AS`
    /// virtual-memory ceiling via a `pre_exec` hook — enforced by the kernel
    /// itself from the moment the child execs, before any component code
    /// runs.
    pub fn harden_before_spawn(command: &mut Command, memory_limit_bytes: Option<u64>) {
        command.process_group(0);
        if let Some(limit) = memory_limit_bytes {
            // Safety: the closure only calls `setrlimit`, an async-signal-safe
            // syscall — no allocation, no locking, satisfying `pre_exec`'s
            // requirement that the closure be safe to run between fork and
            // exec in the child.
            unsafe {
                command.pre_exec(move || {
                    let rlim = libc::rlimit {
                        rlim_cur: limit as libc::rlim_t,
                        rlim_max: limit as libc::rlim_t,
                    };
                    if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
    }

    /// Kills the whole process group `child` leads (its pgid equals its pid
    /// because `harden_before_spawn` called `process_group(0)`) — reaches
    /// grandchildren a misbehaving component spawned, which a plain
    /// `child.kill()` (SIGKILL to the immediate process only) would miss.
    pub fn kill_process_tree(child: &mut Child) {
        unsafe {
            libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = child.kill();
    }
}

pub use imp::{harden_before_spawn, kill_process_tree, JobObject};
