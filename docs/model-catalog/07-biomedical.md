# Domain 07 — Biomedical & Health

*Architecture substrate: longitudinal multimodal patient models (EHR, omics, imaging, wearables, environmental) with temporal consistency, calibration, and clinical-grade uncertainty. Privacy/equity constraints are first-class.*

**1. Digital Twin of Patient** — Continuous personalized model of an individual's health. Design: longitudinal state estimation, intervention simulation, risk forecasting, explainability for clinicians.

**2. Personal Genome Interpreter** — Explains genomic variants in context. Design: variant effect prediction, polygenic scoring, ancestry-aware allele frequencies, evidence grading.

**3. Rare Disease Diagnostician** — Shortens diagnostic odysseys. Design: phenotyping from free text, differential ranking, genotype-phenotype linking, undiagnosed-case learning.

**4. Multi-Omics Integrator** — Fuses genomics/transcriptomics/proteomics/metabolomics. Design: modality alignment, missing-data handling, disease subtype discovery, biomarker panels.

**5. Liquid Biopsy Analyzer** — Detects cancer signals from blood. Design: cfDNA methylation patterns, fragmentomics, variant calling at low allele fraction, longitudinal tracking.

**6. Cancer Evolution Tracker** — Models tumor clonal dynamics. Design: phylogeny inference from sequencing, treatment response modeling, resistance emergence prediction.

**7. Immunotherapy Response Predictor** — Predicts checkpoint-inhibitor benefit. Design: tumor microenvironment features, TMB/MSI integration, immune contexture modeling, trial-validated calibration.

**8. Cell Therapy Designer** — Designs CAR-T and cell therapies. Design: construct design (scFv, spacer, signaling), antigen escape modeling, safety switch design.

**9. Organ-on-Chip Simulator** — Simulates microphysiological systems. Design: fluidics + cell behavior coupling, drug response extrapolation, assay standardization.

**10. Whole-Organ Model** — Physics-based organ simulation. Design: multi-scale coupling (molecular→tissue→organ), patient-specific geometry, functional metrics.

**11. Blood Flow Simulator** — Hemodynamics modeling. Design: CFD with patient anatomy, stenosis/aneurysm risk, surgical planning (bypass/Fontan), thrombus formation.

**12. Cardiac Digital Twin** — Personalized heart model. Design: ECG/Echo/MRI fusion, electromechanical coupling, arrhythmia substrate mapping, ablation planning.

**13. Neurological Disorder Modeler** — Models disease progression (MS, ALS, epilepsy). Design: biomarker trajectory modeling, subtype stratification, treatment timing optimization.

**14. Parkinson's Progression Model** — Tracks motor/non-motor decline. Design: wearable motor scores, medication-response modeling, DBS parameter optimization.

**15. Alzheimer's Risk Stratifier** — Identifies high-risk individuals early. Design: amyloid/tau/neurodegeneration (ATN) staging, cognitive trajectory modeling, prevention trial matching.

**16. Depression Biomarker Finder** — Identifies depression subtypes/biomarkers. Design: multimodal (sleep, speech, actigraphy, imaging) integration, treatment-response subgroups, digital phenotyping.

**17. Anxiety Digital Therapist** — Delivers evidence-based CBT at scale. Design: CBT protocol engines, exposure hierarchy generation, engagement modeling, crisis escalation.

**18. Sleep Architecture Analyzer** — Stages sleep and detects disorders. Design: PSG/wearable fusion, apnea-hypopnea scoring, circadian phase estimation, intervention recommendations.

**19. Pain Level Estimator** — Estimates pain from observable signals. Design: facial/voice/physiology fusion, self-report calibration, chronic pain trajectories, non-verbal populations support.

**20. Rehabilitation Planner** — Designs personalized rehab programs. Design: impairment-based targeting, progress tracking, adherence modeling, adaptive difficulty.

**21. Prosthetic Control Model** — Intuitive limb-prosthesis control. Design: EMG/IMU intent decoding, grip selection, sensory feedback mapping, user adaptation.

**22. Brain-Computer Interface Decoder** — Decodes neural intent for communication/control. Design: invasive/non-invasive fusion, adaptive decoding, calibration efficiency, error-related potentials.

**23. Neuroprosthesis Speech Synthesizer** — Restores speech from neural signals. Design: articulatory kinematics decoding, intelligibility optimization, user voice modeling.

**24. Visual Prosthesis Translator** — Translates scenes for visual implants. Design: salience compression, phosphene rendering, task-based optimization, user training support.

**25. Exoskeleton Balance Controller** — Balances and assists gait. Design: model-predictive control, terrain estimation, fall prevention, metabolic cost reduction.

**26. Telemedicine Triage Engine** — Routes patients to right care level. Design: symptom-based urgency scoring, resource-aware routing, safety netting, follow-up automation.

**27. Symptom Checker** — Differential analysis from reported symptoms. Design: likelihood-ranked differentials, red-flag detection, question selection, honest uncertainty.

**28. Differential Diagnostician** — Clinician-support diagnosis. Design: evidence-weighted reasoning, test recommendation value, Bayesian updating, cognitive-bias countermeasures.

**29. Treatment Planner** — Recommends treatment strategies. Design: guideline-grounded, comorbidity-aware, preference-sensitive, contraindication checking.

**30. Drug Interaction Checker** — Predicts polypharmacy risks. Design: mechanism-based interaction prediction, severity grading, alternative suggestions, renal/hepatic adjustments.

**31. Medication Adherence Monitor** — Detects and supports non-adherence. Design: refill/wearable signals, barrier inference, intervention selection, clinician alerts.

**32. Adverse Event Predictor** — Anticipates drug reactions in individuals. Design: pharmacogenomic + clinical risk modeling, monitoring recommendations, signal detection.

**33. Hospital Readmission Predictor** — Forecasts 30-day readmission. Design: discharge-time risk, transitional-care targeting, intervention effectiveness, fairness audits.

**34. ICU Deterioration Alarm** — Early warning for decompensation. Design: continuous vital modeling, sepsis/ARDS detection, false-alarm reduction, actionable alerts.

**35. Sepsis Early Detector** — Hourly sepsis risk scoring. Design: temporal vital/lab modeling, antibiotic-timing optimization, organ-dysfunction trajectory, decision support.

**36. Surgical Robot Controller** — Assists with precision and safety. Design: tissue interaction modeling, tremor filtering, constraint-based assistance, autonomous subtask execution.

**37. Preoperative Risk Model** — Surgical risk stratification. Design: procedure-specific risk, frailty integration, optimization opportunities, shared decision-making support.

**38. Postoperative Complication Forecaster** — Predicts recovery trajectories. Design: continuous vitals/symptom modeling, early complication detection, discharge readiness, follow-up planning.

**39. Wound Healing Monitor** — Tracks wound progression. Design: imaging-based staging, infection detection, treatment response, tele-wound-care support.

**40. Chronic Disease Manager** — Coordinates long-term condition care. Design: multi-condition care plans, self-management coaching, flare prediction, care-team communication.

**41. Diabetes Glucose Forecaster** — Predicts glucose excursions. Design: CGM + insulin + meals modeling, hypo/hyperglycemia alarms, closed-loop insulin support.

**42. Hypertension Manager** — Optimizes blood pressure control. Design: home BP trajectory modeling, medication titration support, lifestyle intervention targeting.

**43. Longevity Intervention Designer** — Designs evidence-based aging interventions. Design: aging-clock response modeling, intervention stacking, biomarker monitoring, safety constraints.

**44. Aging Clock Calibrator** — Measures biological age from data. Design: epigenetic/clinical/proteomic clocks, clock-consistency, intervention responsiveness, interpretability.

**45. Senolytic Candidate Finder** — Identifies senescent-cell drugs. Design: senescence signatures, compound screening prioritization, safety modeling.

**46. Regenerative Medicine Planner** — Plans tissue regeneration strategies. Design: scaffold/biomaterial selection, growth factor regimes, stem cell protocols, vascularization planning.

**47. Stem Cell Differentiation Director** — Directs differentiation protocols. Design: protocol optimization, media composition, quality prediction, scale-up readiness.

**48. Personalized Nutrition Engine** — Individualized dietary guidance. Design: gut microbiome + genetics + glucose responses, meal planning, behavior change support, long-term adherence.

**49. Microbiome Diet Advisor** — Diet recommendations via microbiome. Design: microbial ecology modeling, dietary response prediction, prebiotic/probiotic selection, longitudinal tracking.

**50. Mental Health Crisis Detector** — Detects acute risk signals. Design: multimodal passive sensing, linguistic risk markers, escalation protocols, crisis-resource routing.

---
*Generated 2026-08-01 · BrainBuilder Model Catalog · Domain 07 of 20*
