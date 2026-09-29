"""DSpark: a speculative-decoding serving engine (drafter + verifier).

This is the *serving-time* counterpart to BrainBuilder's training graphs. The
same drafting heads exist as trainable BrainBuilder components
(`components/{parallel_intern,low_rank_markov_head,confidence_head}.edn`); this
package assembles them into the online generation loop — hardware-aware draft
sizing (SPS), confidence-gated early termination, and rejection-sampling
verification — so a user can build a DeepSeek-style speculative decoder.

Nothing here is wired into BrainBuilder's trainer: training a model and serving
it speculatively are distinct concerns, kept decoupled on purpose.
"""

from .config import DSparkConfig
from .dynamic_hardware import SPSManager
from .neural_components import ParallelIntern, LowRankMarkovHead, ConfidenceHead
from .engine import SpeculativeEngine

__all__ = [
    "DSparkConfig",
    "SPSManager",
    "ParallelIntern",
    "LowRankMarkovHead",
    "ConfidenceHead",
    "SpeculativeEngine",
]
