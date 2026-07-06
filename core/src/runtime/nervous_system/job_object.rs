//! OS-level process containment, layered on top of the portable
//! capability/timeout checks in `supervisor.rs`. Windows: a real Job Object
//! with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (dropping the handle kills the
//! whole process tree — covers grandchildren a plain `Child::kill()` misses)
//! and an optional process memory ceiling. Linux (and other non-Apple Unix):
//! a real `RLIMIT_AS` ceiling applied in the child before it execs
//! (`harden_before_spawn`, called ahead of `spawn()` — a Job Object's limits
//! can be set after spawn, but rlimits can't), plus a private process group
//! so `kill_process_tree` can `killpg` every descendant instead of only the
//! immediate child; proven on a real Linux CI runner by
//! `supervisor::tests::rlimit_as_ceiling_is_enforced_by_the_kernel_on_unix`
//! and `..::kill_process_tree_reaches_a_grandchild_via_the_process_group`.
//! macOS: the same `RLIMIT_AS` + process-group containment, *plus* a real
//! `sandbox-exec` profile built from the runtime's `Capabilities` — the
//! child can only read the paths it was actually granted (plus the minimal
//! system paths a dynamically-linked executable needs to start at all), and
//! network is denied unless the capability grant allows it. This is the
//! macOS counterpart to the Linux path-check + rlimit story, using the same
//! deny-by-default `Capabilities` model rather than a separate mechanism.
//! cgroups/seccomp hardening beyond a memory rlimit and a group kill (Linux),
//! or beyond the sandbox profile above (macOS), remains a real follow-up.

use super::Capabilities;

#[cfg(windows)]
mod imp {
    use super::Capabilities;
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
    pub fn harden_before_spawn(_command: &mut Command, _memory_limit_bytes: Option<u64>, _caps: &Capabilities) {}

    /// A direct kill of the immediate child. Grandchildren are reaped by the
    /// assigned Job Object's kill-on-close semantics once `_job` (held by
    /// the caller) drops — this call handles the process that's actually
    /// hung, which the Job Object teardown alone doesn't immediately signal.
    pub fn kill_process_tree(child: &mut Child) {
        let _ = child.kill();
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Capabilities;
    use std::ffi::OsString;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};

    pub struct JobObject;

    impl JobObject {
        /// Unix/macOS hardening (rlimit + process group + sandbox profile)
        /// must be applied *before* `spawn()` — see `harden_before_spawn`,
        /// called ahead of it. There is nothing left to attach post-spawn,
        /// unlike the Windows Job Object.
        pub fn new(_memory_limit_bytes: Option<u64>) -> Option<Self> {
            None
        }
        pub fn assign(&self, _child: &Child) -> bool {
            false
        }
    }

    /// Rewraps `command` to run under a real `sandbox-exec` profile built
    /// from `caps` (deny-by-default: only `caps.read_paths()` plus the
    /// minimal system paths a dynamically-linked executable needs to start
    /// are readable; network denied unless `caps.network_allowed()`), then
    /// applies the same process-group + `RLIMIT_AS` hardening as other Unix
    /// targets as defense in depth alongside the sandbox profile.
    ///
    /// `sandbox-exec -p <profile> -- <program> <args>` applies the profile
    /// via `sandbox_init` and then `execve`s the target *in the same
    /// process* (no extra fork) — transparent to whatever stdio/env the
    /// caller has already configured on `command`, regardless of whether
    /// that configuration happens before or after this call.
    pub fn harden_before_spawn(command: &mut Command, memory_limit_bytes: Option<u64>, caps: &Capabilities) {
        let profile = sandbox_profile(caps);
        let program = command.get_program().to_os_string();
        let args: Vec<OsString> = command.get_args().map(|a| a.to_os_string()).collect();

        let mut wrapped = Command::new("sandbox-exec");
        wrapped.arg("-p").arg(profile).arg("--").arg(program).args(args);
        *command = wrapped;

        // Same containment as the generic Unix path (see the non-macOS Unix
        // `imp` module below): private process group for a whole-tree kill,
        // and a pre-exec RLIMIT_AS ceiling.
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

    /// A minimal, real Seatbelt profile: deny everything by default, then
    /// explicitly allow just enough for a normal dynamically-linked
    /// executable to start (dyld/libSystem/the OS's shared libraries),
    /// process exec/fork (`sandbox-exec` itself execs into the target; the
    /// target may itself fork/exec, e.g. a shell test script), and reads of
    /// exactly the paths this runtime's capability grant names. Anything not
    /// listed here — including all filesystem writes and, unless
    /// `caps.network_allowed()`, all network access — falls through to the
    /// default deny.
    fn sandbox_profile(caps: &Capabilities) -> String {
        let mut lines = vec![
            "(version 1)".to_string(),
            "(deny default)".to_string(),
            "(allow process-exec*)".to_string(),
            "(allow process-fork)".to_string(),
            "(allow signal (target self))".to_string(),
            "(allow sysctl-read)".to_string(),
            "(allow mach-lookup)".to_string(),
            "(allow file-read* \
                (subpath \"/usr\") (subpath \"/bin\") (subpath \"/sbin\") \
                (subpath \"/System\") (subpath \"/private/var/db/dyld\") \
                (literal \"/dev/null\") (literal \"/dev/zero\") \
                (literal \"/dev/random\") (literal \"/dev/urandom\") \
                (literal \"/dev/dtracehelper\"))"
                .to_string(),
        ];
        for path in caps.read_paths() {
            let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
            lines.push(format!("(allow file-read* (subpath {}))", quote(&canon.to_string_lossy())));
        }
        if caps.network_allowed() {
            lines.push("(allow network*)".to_string());
        }
        lines.join("\n")
    }

    /// Sandbox-profile string literal quoting: backslash- and
    /// quote-escape so a path containing either can't break out of the
    /// `(subpath "...")` term it's embedded in.
    fn quote(s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }

    /// Kills the whole process group `child` leads (its pgid equals its pid
    /// because `harden_before_spawn` called `process_group(0)`) — reaches
    /// grandchildren a misbehaving component spawned, which a plain
    /// `child.kill()` (SIGKILL to the immediate process only) would miss.
    /// Note: the immediate child here is `sandbox-exec`, which has already
    /// `execve`'d into the real target by the time this can run, so its pid
    /// (and thus the process group) is the real target's, not a wrapper's.
    pub fn kill_process_tree(child: &mut Child) {
        unsafe {
            libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = child.kill();
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::Capabilities;
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
    /// runs. `caps` isn't consulted here: on non-Apple Unix targets there is
    /// no OS-level sandbox profile (that's macOS's `sandbox-exec`
    /// counterpart, in the `imp` module above) — path enforcement instead
    /// happens portably, in `Supervisor::run_checked_named`'s
    /// `caps.check_read` calls before this ever runs.
    pub fn harden_before_spawn(command: &mut Command, memory_limit_bytes: Option<u64>, _caps: &Capabilities) {
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
