"""SPS (System Performance vs Speed) manager: picks the draft length that
balances throughput against server stability from live hardware load.

Reads GPU utilization via NVML when present, and degrades gracefully to a CPU
reading when the NVIDIA driver/library is unavailable — a missing driver must
never crash the serving path. Every fallback still logs a warning once, so a
degraded reading is visible in logs rather than silently masquerading as a
real GPU number.
"""
import logging

logger = logging.getLogger(__name__)

try:
    import psutil
    _HAS_PSUTIL = True
except Exception as e:  # ImportError, or a broken partial install
    _HAS_PSUTIL = False
    logger.warning("psutil unavailable (%s) — SPS will use a fixed mid-point load estimate instead of real CPU load.", e)

try:
    import pynvml
    _HAS_NVML = True
except Exception as e:  # ImportError, or a broken partial install
    _HAS_NVML = False
    logger.warning("pynvml unavailable (%s) — SPS will use a CPU-utilization proxy instead of real GPU load.", e)


class SPSManager:
    def __init__(self, min_len, max_len, critical_load, idle_load):
        self.min_len = min_len
        self.max_len = max_len
        self.critical_load = critical_load
        self.idle_load = idle_load
        self._nvml_ready = False
        self._warned_util_failure = False
        if _HAS_NVML:
            try:
                pynvml.nvmlInit()
                self._nvml_ready = True
            except Exception as e:
                # Library present but no usable device/driver — fall back.
                self._nvml_ready = False
                logger.warning("pynvml.nvmlInit() failed (%s) — SPS will use a CPU-utilization proxy instead of real GPU load.", e)

    def close(self):
        """Releases the NVML handle acquired in `__init__`. NVML init/shutdown
        is a real resource pair (an open driver handle), so an `SPSManager`
        that's created and discarded repeatedly (e.g. one per request/session
        in a longer-lived server) would otherwise leak a handle each time."""
        if self._nvml_ready:
            try:
                pynvml.nvmlShutdown()
            except Exception as e:
                logger.warning("pynvml.nvmlShutdown() failed (%s) — ignoring, process is shutting down anyway.", e)
            finally:
                self._nvml_ready = False

    def __del__(self):
        # Best-effort: guarantees the handle is released even if the owner
        # forgets to call close() explicitly (e.g. on an exception path).
        self.close()

    def utilization(self):
        """GPU utilization in [0, 1], or a CPU-based proxy when no GPU is
        readable. Any NVML hiccup returns a safe mid-point rather than raising,
        but logs a warning the first time it happens (not on every call — this
        runs once per generation step, so per-call logging would spam)."""
        if not self._nvml_ready:
            if not _HAS_PSUTIL:
                return 0.5
            return psutil.cpu_percent(interval=None) / 100.0
        try:
            handle = pynvml.nvmlDeviceGetHandleByIndex(0)
            return pynvml.nvmlDeviceGetUtilizationRates(handle).gpu / 100.0
        except Exception as e:
            if not self._warned_util_failure:
                logger.warning("NVML utilization read failed (%s) — falling back to a fixed mid-point load estimate.", e)
                self._warned_util_failure = True
            return 0.5

    def draft_length(self, active_requests):
        """Allowed draft block size given load. Heavy load shortens drafts to
        protect the server; idle load lengthens them to maximize speed; the
        mid-range interpolates linearly. Always within [min_len, max_len]."""
        util = self.utilization()

        if util > self.critical_load or active_requests > 100:
            return self.min_len
        if util < self.idle_load and active_requests < 20:
            return self.max_len

        span = max(self.critical_load - self.idle_load, 1e-6)
        ratio = (util - self.idle_load) / span
        adjusted = int(round(self.max_len - ratio * (self.max_len - self.min_len)))
        return max(self.min_len, min(self.max_len, adjusted))
