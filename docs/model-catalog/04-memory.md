# Domain 04 — Memory & Knowledge

*Architecture substrate: layered memory hierarchy (working → episodic → semantic → procedural) with write-ahead consolidation, provenance, and forgetting policies.*

**1. Infinite Context Memory** — Retains unbounded context with bounded resources. Design: hierarchical summarization + retrieval, KV-cache eviction, salience-weighted retention, reconstruction on demand.

**2. Hierarchical Memory Manager** — Organizes memories into abstraction levels. Design: memory tree with consolidation upward, query routing across levels, forgetting at leaf level first.

**3. Sparse Distributed Memory** — Holographic-like associative storage. Design: sparse codes with cleanup, noise-robust readout, capacity estimation, online writes.

**4. Episodic Story Compressor** — Compresses experiences into retrievable narrative form. Design: event segmentation, salience ranking, lossy-but-faithful compression, reconstruction with detail control.

**5. Semantic Knowledge Graph Builder** — Extracts durable facts from experience. Design: open information extraction, entity resolution, relation validation, contradiction detection.

**6. Procedural Skill Memory** — Stores how-to knowledge for reuse. Design: skill programs with preconditions, execution traces, generalization across instances, failure annotations.

**7. Muscle Memory Encoder** — Distills repeated motor patterns. Design: trajectory clustering, reflex-level caching, execution with low deliberation, adaptation under perturbation.

**8. Associative Recall Engine** — Cue-driven memory retrieval. Design: content-addressable lookup, spreading activation, partial-cue completion, false-association guard.

**9. Forgetting Curve Optimizer** — Schedules review to maximize retention. Design: Ebbinghaus-style decay models, per-item difficulty, spacing optimization, retrieval practice scheduling.

**10. Memory Consolidation Scheduler** — Converts labile to stable memories. Design: replay prioritization, offline consolidation passes, interference resolution, sleep-like restructuring.

**11. Sleep-Like Replay Engine** — Offline rehearsal of important experiences. Design: experience replay with importance sampling, complementary learning systems, abstraction during replay.

**12. Memory Pruning Strategist** — Removes low-value memories safely. Design: value scoring, forgetting with provenance, reconstruction check, capacity budgeting.

**13. Knowledge Base Synthesizer** — Merges sources into a coherent KB. Design: schema inference, conflict resolution, redundancy elimination, consistency enforcement.

**14. Ontology Builder** — Constructs formal domain ontologies. Design: concept/relation induction, axiom suggestion, alignment with upper ontologies, modularization.

**15. Taxonomy Auto-Constructor** — Builds classification hierarchies. Design: hierarchical clustering with labels, parent-child validation, navigation utility scoring.

**16. Fact Freshness Manager** — Tracks when facts expire or update. Design: temporal validity modeling, source monitoring, staleness alarms, versioned facts.

**17. Knowledge Provenance Tracker** — Records where every fact came from. Design: source graphs, trust weighting, citation chains, verifiability scoring.

**18. Contradiction Resolver** — Detects and reconciles conflicting knowledge. Design: inconsistency detection, source arbitration, context disambiguation, revision proposals.

**19. Knowledge Migration Engine** — Transfers knowledge between representations. Design: schema mapping, semantic translation, integrity-preserving transforms, migration rollback.

**20. Cross-Lingual Knowledge Aligner** — Aligns knowledge across languages. Design: multilingual embeddings, entity linking across corpora, translation-consistency checks.

**21. Cultural Knowledge Model** — Encodes culturally-specific norms and references. Design: cultural taxonomies, context sensitivity, taboos and customs modeling, localization support.

**22. Tacit Knowledge Extractor** — Surfaces implicit expertise. Design: expert elicitation interviews, behavioral cloning probes, articulable-rule distillation.

**23. Expert Interviewer** — Runs structured knowledge elicitation. Design: question generation, follow-up selection, consistency cross-checking, capture of heuristics.

**24. Institutional Memory Archivist** — Preserves organizational knowledge. Design: document/chat/wiki ingestion, role-annotated memory, retention policies, retrieval interfaces.

**25. Legal Precedent Retriever** — Finds relevant case law. Design: legal embedding space, citation graph traversal, holding/ratio extraction, jurisdiction filtering.

**26. Scientific Literature Synthesizer** — Distills bodies of papers into structured knowledge. Design: claim extraction, evidence grading, contradiction mapping, gap identification.

**27. Historical Event Reasoner** — Models historical causality and contingency. Design: event chronologies, counterfactual history, source triangulation, period-context reasoning.

**28. Personal Memory Assistant** — Helps individuals remember. Design: lifelog ingestion, reminiscence triggers, fact verification, privacy-preserving local storage.

**29. Lifelog Analyzer** — Derives insight from continuous personal data. Design: activity segmentation, habit discovery, anomaly detection, summary generation.

**30. Dream Interpreter (memory replay visualization)** — Visualizes memory replay patterns. Design: latent replay decoding, symbolic imagery mapping, narrative generation with epistemic honesty.

**31. Collective Memory Model** — Models society-level remembering/forgetting. Design: cultural memory dynamics, media influence, generational shifts, commemoration patterns.

**32. Oral Tradition Preserver** — Digitizes and structures oral knowledge. Design: speech transcription for heritage, narrative structure preservation, speaker attribution, lossless archival.

**33. Language Death Reviver** — Helps revitalize endangered languages. Design: low-resource grammar induction, dictionary reconstruction, immersion materials generation, speaker community tools.

**34. Dead Script Decoder** — Deciphers undeciphered writing systems. Design: statistical cryptanalysis, pattern mining, bilingual text leverage, archaeolinguistic constraints.

**35. Artefact Provenance Analyzer** — Traces object origins and chains of custody. Design: stylistic dating, materials fingerprinting, forgery detection, collection history modeling.

**36. Archaeological Site Model** — Reconstructs sites from excavation data. Design: stratigraphy reasoning, artifact spatial analysis, site formation processes, visualization.

**37. Paleontological Reasoner** — Reconstructs extinct organisms and ecosystems. Design: fossil gap filling, phylogenetic inference, biomechanics estimation, taphonomy correction.

**38. Geological Time Mapper** — Correlates strata and events across time. Design: chronostratigraphy alignment, dating method fusion, event ordering, uncertainty propagation.

**39. Evolutionary History Reconstructor** — Infers phylogenies and trait evolution. Design: molecular phylogenetics, trait ancestral state inference, divergence dating, reticulation detection.

**40. Genealogical Network Analyzer** — Builds and queries family trees. Design: record linkage, kinship inference, genetic genealogy integration, privacy controls.

**41. Organizational Knowledge Grapher** — Maps who-knows-what in an organization. Design: expertise extraction, collaboration graphs, skill gap analysis, succession planning.

**42. Wiki Consolidator** — Merges and de-duplicates wiki content. Design: page similarity, content conflict resolution, template harmonization, link repair.

**43. FAQ Automator** — Generates FAQ knowledge from support content. Design: question extraction, answer pairing, intent clustering, freshness maintenance.

**44. Manual-to-Knowledge Converter** — Transforms manuals into structured knowledge. Design: section understanding, procedure extraction, troubleshooting trees, media alignment.

**45. Tribal Knowledge Saver** — Captures knowledge at risk of being lost. Design: exit interviews, shadowing analysis, documentation generation, criticality prioritization.

**46. Onboarding Knowledge Packager** — Creates learning paths for newcomers. Design: prerequisite chains, role-specific curricula, knowledge checks, mentorship pairing.

**47. Knowledge Decay Auditor** — Flags stale or wrong knowledge. Design: usage monitoring, contradiction detection, source staleness, revalidation workflows.

**48. Memory-Aid Optimizer (spaced repetition)** — Personalizes review schedules. Design: memory strength models, item difficulty dynamics, interleaving, retrieval-context variation.

**49. Collective Intelligence Amplifier** — Enhances group knowledge work. Design: idea clustering, consensus tracking, minority-report preservation, structured deliberation.

**50. Crystallized Intelligence Model** — Applies accumulated knowledge to novel problems. Design: analogy-rich retrieval, schema application, knowledge-grounded reasoning, wisdom metrics.

---
*Generated 2026-08-01 · BrainBuilder Model Catalog · Domain 04 of 20*
