# Domain 01 — Core ML & Foundation Models

*Architecture substrate: modular foundation model with steerable latent space, pluggable memory, verifier loops, and conditional compute. All models below are Tauri/Rust-embeddable in the BrainBuilder pattern (pub struct + pub async fn commands + widget registry).*

**1. Universal Continual Learner** — Learns a lifetime of tasks without catastrophic forgetting. Design: dual-memory (fast weights + slow consolidation), replay synthesis instead of raw buffer storage, EWC-style regularization with online importance estimation.

**2. Meta-Architecture Search Engine** — Discovers novel neural topologies outperforming hand-designed ones. Design: differentiable NAS over an op library (conv, attention, recurrent, memory cells) with latency/energy constraints as secondary objectives.

**3. Self-Modifying Weights Model** — Weights that can rewrite themselves under hard safety constraints. Design: gradient-gated rewrite heads, monotonic safety invariants checked before each mutation, revert ledger.

**4. Emergent Capability Predictor** — Forecasts which capabilities appear at which scale, before training. Design: power-law extrapolation across capability probes, cross-checked against ablations on smaller runs.

**5. Interpretability Transducer** — Maps any internal activation state to human-readable concepts. Design: sparse autoencoders over residual streams, concept dictionary with causal intervention validation.

**6. Mechanistic Interpretability Analyzer** — Finds circuits and algorithms inside trained networks. Design: activation patching, attention head clustering, IOI-style circuit discovery with automated circuit summarization.

**7. Causal World Model** — Learns the true causal graph of its environment, not just correlations. Design: do-calculus guided structure learning, interventional data querying, counterfactual consistency checks.

**8. Abstraction Hierarchies Learner** — Builds multi-level abstractions automatically. Design: recursive compression with information bottleneck, level-jumping transfer, abstraction reuse across tasks.

**9. Compositional Generalizer** — Recombines known skills to solve novel tasks. Design: skill embedding space with compositional operators, generalization via latent interpolation, OOD task probes.

**10. Symmetry Exploiter** — Detects and exploits invariances/equivariances in data. Design: automatic group discovery (translations, rotations, permutations), group-equivariant layer injection, data-efficiency gains as metric.

**11. Sample Efficiency Booster** — Learns from a handful of examples via strong priors. Design: hierarchical Bayes over task families, prior-then-finetune with rapid adaptation, meta-learned initializations.

**12. Uncertainty Decomposer** — Splits uncertainty into aleatoric, epistemic, and structural components. Design: ensemble + input-noise decomposition, separate heads per uncertainty type, calibrated confidence interfaces.

**13. Out-of-Distribution Guardian** — Detects distribution shift at input, feature, and label level. Design: density estimation in latent space, energy-based OOD scores, drift alarms with attribution.

**14. Active Query Selector** — Chooses the most informative data to label next. Design: BALD-style acquisition with cost constraints, batch diversity penalties, human-in-the-loop API.

**15. Curriculum Architect** — Designs learning curricula that maximize final performance. Design: task difficulty modeling, dynamic pacing, self-play style ordering, curriculum transfer across learners.

**16. Reward Designer** — Constructs reward functions for RL from high-level intent. Design: reward shaping from demonstrations, potential-based shaping guarantees, reward model ensemble with preference queries.

**17. Inverse Reward Learner** — Infers goals and rewards from observed behavior. Design: maximum entropy IRL, option-based reward decomposition, reward ambiguity quantification.

**18. Preference Refiner** — Turns noisy, inconsistent human feedback into clean objectives. Design: Bradley-Terry models with noise models, disagreement clustering, interactive preference elicitation.

**19. Transfer Bridge** — Zero-shot task transfer via shared representation spaces. Design: task-agnostic embedding space with metric alignment, task arithmetic in embedding space, negative transfer detection.

**20. Multi-Task Consolidator** — Learns shared representations across tasks without interference. Design: task-specific routing heads over shared trunk, gradient surgery (PCGrad), conflict resolution via projection.

**21. Modality Harmonizer** — Unified multimodal embedding space (text/image/audio/sensor/tabular). Design: contrastive alignment, missing-modality robustness, cross-modal generation bridge.

**22. Sparse Activation Engine** — Conditional computation at scale, activating only relevant pathways. Design: mixture-of-experts with learned routing, load-balancing losses, expert pruning and merging.

**23. Recurrent World Simulator** — Predicts environment rollouts for planning. Design: latent dynamics model (Dreamer-style), imagination-based planning, model-based value estimation.

**24. Intrinsic Motivation Engine** — Curiosity- and novelty-driven exploration. Design: prediction-error curiosity, count-based novelty in latent space, empowerment maximization.

**25. Skill Library Manager** — Organizes, indexes, and retrieves reusable skills. Design: skill embeddings, hierarchical skill taxonomies, skill composition and versioning, drift detection.

**26. Attention Architect** — Designs novel attention variants for given tasks. Design: search over attention mechanisms (linear, sliding-window, sparse, state-space), theoretical cost modeling, empirical ranking.

**27. Tokenizer Designer** — Finds optimal tokenizations for data and compute budgets. Design: BPE variants with loss-aware merging, multi-lingual tokenizer calibration, compression-rate vs model-quality tradeoff.

**28. Long-Context Compressor** — Lossy-but-useful context distillation for unbounded inputs. Design: hierarchical summarization with query-relevance weighting, KV-cache eviction policies, faithful recall guarantees.

**29. Streaming Learner** — Online learning without replay buffers or catastrophic drift. Design: continual backprop alternatives, adaptive learning rates per parameter, change-point detection, bounded memory footprints.

**30. Data Collator** — Merges heterogeneous datasets into coherent training corpora. Design: deduplication, schema unification, quality scoring, curriculum ordering, leakage prevention.

**31. Benchmark Proposer** — Invents valid, non-gaming new tasks and benchmarks. Design: task generation with verifiable answers, difficulty calibration, adversarial example filtering, human-verifiable validation sets.

**32. Failure Mode Explorer** — Adversarial search for model weaknesses. Design: red-team generation, gradient-based input perturbation, coverage-guided fuzzing of latent space, structured failure reports.

**33. Scaling Law Fitter** — Predicts compute/data/parameter tradeoffs before runs. Design: power-law curve fitting, compute-optimal frontier (Chinchilla-style), extrapolation with confidence intervals.

**34. Resource-Adaptive Model** — Degrades gracefully under compute, memory, or latency limits. Design: anytime inference (early exits), dynamic depth/width selection, quantization on demand, graceful degradation contracts.

**35. Federated Coordination Engine** — Trains across organizations without sharing raw data. Design: federated averaging variants, heterogeneity handling, communication compression, differential privacy noise calibration.

**36. Privacy-Preserving Trainer** — DP training at scale with usable utility. Design: DP-SGD with adaptive clipping, RDP accounting, public-pretrain + private-finetune hybrid, utility-preserving noise schedules.

**37. Federated Personalization** — Per-user adaptation within federated systems. Design: personalization layers (Per-FedAvg), meta-learning across clients, privacy-safe user embeddings.

**38. Model Fusion Engine** — Merges models without retraining. Design: weight interpolation (model soups), task-vector arithmetic, architecture-compatible fusion, interference detection.

**39. Knowledge Editing Surgeon** — Surgical weight updates to change specific facts. Design: locate-and-edit (ROME-style), rank-one updates, editing scope control, side-effect measurement.

**40. Unlearning Engine** — Removes specific knowledge/data from models on request. Design: gradient ascent on target data, scrubbing with provenance tracking, residual knowledge audits, certified unlearning bounds.

**41. Verification Oracle** — Checks model outputs for correctness against ground truth or rules. Design: verifier ensembles, consistency checks, external tool calls (code exec, math solvers), confidence-gated verification.

**42. Self-Consistency Enhancer** — Reduces hallucination via internal agreement. Design: multi-sample consistency scoring, entropy-based abstention, contrastive decoding, fact-grounded generation.

**43. Token Budget Planner** — Allocates reasoning compute optimally. Design: adaptive thinking tokens, difficulty estimation, compute-accuracy frontier, budget-aware early stopping.

**44. Test-Time Compute Scheduler** — Decides how much computation to spend per input. Design: adaptive depth/ensemble/best-of-n, cost-sensitive decisions, latency-aware routing.

**45. Ensemble Composer** — Builds optimal model committees. Design: diversity-aware selection, stacking with calibration, dynamic weighting by input, ensemble pruning.

**46. Distillation Architect** — Teacher-student distillation at scale. Design: logit/feature/relation distillation, multi-teacher aggregation, distillation-aware architecture selection.

**47. Hardware-Aware Architect** — NAS constrained to specific chips. Design: hardware cost models (FLOPs, memory, bandwidth), Pareto search over accuracy/cost, deployment-target specialization.

**48. Energy-Aware Trainer** — Carbon- and energy-efficient training. Design: energy budgets, checkpoint scheduling on price signals, mixed-precision and sparse training, carbon accounting.

**49. Provable Bounds Estimator** — Certified robustness guarantees. Design: Lipschitz bound computation, randomized smoothing, abstract interpretation (DeepPoly), certified radius reporting.

**50. AGI Safety Evaluator** — Measures alignment, deception, and risk of advanced systems. Design: multi-stressor evaluation suites, capability vs alignment tracking, adversarial roleplay probes, safety case drafting.

---
*Generated 2026-08-01 · BrainBuilder Model Catalog · Domain 01 of 20*
