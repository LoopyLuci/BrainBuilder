# BrainBuilder Model Catalog — 1000 Models for the Next 100 Years

> **Purpose.** A planned and designed catalog of **1,000 unique models** spanning every field and task class likely to matter over the next century. Each entry is a design brief: a name, a purpose, and the core architectural approach. The catalog is a *design document* — the starting point for implementation batches (Rust modules + Tauri commands + React widgets in the BrainBuilder pattern, or Python/other stacks where appropriate).

## Design Methodology

1. **Domain-first taxonomy.** 20 domains × 50 models each = 1,000. Domains were chosen to cover the full human+AI activity surface: the stack itself (01), perception (02), cognition (03), memory (04), creation (05), science (06), health (07), planet (08), energy (09), space (10), matter (11), embodiment (12), food (13), wealth (14), society (15), learning (16), communication (17), security (18), the quantum/frontier (19), and long-horizon/existential futures (20).
2. **Gap-filling, not gap-avoidance.** The catalog deliberately reaches into under-served spaces: longevity, space agriculture, mycelium materials, coral restoration, asteroid mining, deep-sea exploration, endangered-language revival, undeciphered-script decoding, sensory restoration, whole-brain emulation, and existential-risk governance. Where an obvious model already exists in the wild (e.g., protein folding), the entry is the *next-generation* design, not a clone.
3. **Architecture substrate per domain.** Each domain file opens with its shared substrate (e.g., physics-informed emulators for climate, neuro-symbolic hybrids for cognition, adversarial-first pipelines for security). This keeps the 50 entries coherent and implementation-ready.
4. **One design sentence per model.** Every entry pairs a purpose with the key design idea (architecture, mechanism, or constraint). This is enough to scope an implementation batch of 5–10 models at a time.
5. **Uncertainty and safety are first-class.** High-stakes domains (society, security, existential) carry explicit safeguards in their substrates — fairness audits, due-process safeguards, adversarial stress-testing, falsification paths.

## Catalog Index (20 domains × 50 = 1,000)

| # | Domain | File | Coverage |
|---|--------|------|----------|
| 01 | Core ML & Foundation Models | [01-core-ml.md](01-core-ml.md) | Continual learning, interpretability, causality, scaling, safety evaluation |
| 02 | Perception & Sensing | [02-perception.md](02-perception.md) | Vision, neuromorphic, bio-signals, spectroscopy, quantum sensors, grav waves |
| 03 | Cognition & Reasoning | [03-cognition.md](03-cognition.md) | Neuro-symbolic, causal, game theory, ethics, whole-brain emulation |
| 04 | Memory & Knowledge | [04-memory.md](04-memory.md) | Episodic/semantic/procedural memory, knowledge graphs, cultural preservation |
| 05 | Generative Creation | [05-generative.md](05-generative.md) | Worlds, music, film, games, design disciplines, art forms |
| 06 | Scientific Discovery | [06-science.md](06-science.md) | Hypothesis loops, replication, drug discovery, synthetic biology |
| 07 | Biomedical & Health | [07-biomedical.md](07-biomedical.md) | Digital twins, diagnostics, BCI, longevity, mental health |
| 08 | Climate & Environment | [08-climate.md](08-climate.md) | Earth digital twin, extremes, biodiversity, carbon, adaptation |
| 09 | Energy & Infrastructure | [09-energy.md](09-energy.md) | Grids, storage, fusion, hydrogen, urban systems |
| 10 | Space & Planetary | [10-space.md](10-space.md) | Launch, navigation, ISRU, rovers, exoplanets, SETI, Fermi |
| 11 | Materials & Chemistry | [11-materials.md](11-materials.md) | Discovery, catalysts, polymers, batteries, multiscale simulation |
| 12 | Robotics & Physical AI | [12-robotics.md](12-robotics.md) | Locomotion, manipulation, swarms, sim-to-real, safety certification |
| 13 | Agriculture & Food | [13-agriculture.md](13-agriculture.md) | Precision ag, livestock, aquaculture, food tech, zero-waste |
| 14 | Economics & Finance | [14-economics.md](14-economics.md) | Macro, credit, markets, DeFi, insurance, UBI simulation |
| 15 | Society & Governance | [15-society.md](15-society.md) | Policy, law, conflict, housing, elections, democratic innovation |
| 16 | Education & Human Development | [16-education.md](16-education.md) | Adaptive tutoring, accessibility, career, lifelong learning |
| 17 | Communication & Media | [17-media.md](17-media.md) | Translation, authenticity, fact-checking, deepfake defense |
| 18 | Cybersecurity & Trust | [18-security.md](18-security.md) | Vulnerability research, malware, forensics, quantum crypto, insurance |
| 19 | Quantum & Frontier Physics | [19-quantum.md](19-quantum.md) | QEC, VQE, QML, dark matter, black holes, unification |
| 20 | Long-Horizon & Existential | [20-longhorizon.md](20-longhorizon.md) | AGI safety, x-risk, post-scarcity, megastructures, civilization futures |

## How to Use This Catalog

- **Scoping implementation batches:** pick a domain file, take 5–10 entries, follow the BrainBuilder integration checklist (Rust module → `mod` + command wiring → smoke test → React panel → `builtins.ts` registration → `npm run build`). Domains 01–03 and 12 map most directly onto BrainBuilder's existing next-gen model stack.
- **Deep-diving a model:** each entry's design sentence names the mechanism to research first (e.g., "sparse autoencoders over residual streams" for the Interpretability Transducer).
- **Auditing coverage:** the 20-domain table doubles as a gap map. If a future task class appears that is not covered, it slots into the nearest domain as model #51+.

## Verification

- 20 files × 50 entries = **1,000 models** (each entry numbered 1–50 within its domain).
- Every entry: name + purpose + design note.
- Generated 2026-08-01 under `docs/model-catalog/` in the BrainBuilder repo.
