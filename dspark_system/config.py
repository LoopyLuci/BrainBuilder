"""Single source of truth for DSpark's limits and hyperparameters."""
from dataclasses import dataclass


@dataclass
class DSparkConfig:
    vocab_size: int = 32000
    hidden_dim: int = 1024
    # Draft-length bounds the SPS manager interpolates between under load.
    max_draft_len: int = 16
    min_draft_len: int = 2
    # Low-rank bottleneck for the Markov suffix-decay corrector.
    markov_rank: int = 32
    # Confidence below this terminates a draft early.
    confidence_threshold: float = 0.60
    # SPS (System Performance vs Speed) curve hardware thresholds.
    gpu_critical_load: float = 0.85
    gpu_idle_load: float = 0.40
