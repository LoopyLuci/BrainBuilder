"""SPS (System Performance vs Speed) manager: picks the draft length that
balances throughput against server stability from live hardware load.

Reads GPU utilization via NVML when present, and degrades gracefully to a CPU
reading when the NVIDIA driver/library is unavailable — a missing driver must
never crash the serving path.
"""
import psutil

try:
    import pynvml
    _HAS_NVML = True
except Exception:  # ImportError, or a broken partial install
    _HAS_NVML = False


class SPSManager:
    def __init__(self, min_len, max_len, critical_load, idle_load):
        self.min_len = min_len
        self.max_len = max_len
        self.critical_load = critical_load
        self.idle_load = idle_load
        self._nvml_ready = False
        if _HAS_NVML:
            try:
                pynvml.nvmlInit()
                self._nvml_ready = True
            except Exception:
                # Library present but no usable device/driver — fall back.
                self._nvml_ready = False

    def utilization(self):
        """GPU utilization in [0, 1], or a CPU-based proxy when no GPU is
        readable. Any NVML hiccup returns a safe mid-point rather than raising."""
        if not self._nvml_ready:
            return psutil.cpu_percent(interval=None) / 100.0
        try:
            handle = pynvml.nvmlDeviceGetHandleByIndex(0)
            return pynvml.nvmlDeviceGetUtilizationRates(handle).gpu / 100.0
        except Exception:
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
