# Domain 02 — Perception & Sensing

*Architecture substrate: sensor-agnostic encoders with time-sync fusion, physics-informed priors, and confidence-calibrated output heads. All models expose a uniform Percept<T> interface (raw → feature → event → decision).*

**1. Universal Sensor Fusion Engine** — Fuses heterogeneous sensors (camera/LiDAR/radar/IMU/thermal) into one world state. Design: cross-modal attention with timestamp alignment, uncertainty-weighted fusion, missing-sensor robustness.

**2. Event-Camera Interpreter** — Extracts motion and structure from asynchronous event streams. Design: spike-based temporal convolutions, event-to-frame reconstruction, high-speed motion estimation.

**3. Neuromorphic Vision Processor** — Event-driven vision with spiking neural networks. Design: SNN layers with surrogate gradients, energy-efficient temporal coding, latency-optimized inference.

**4. 3D Scene Reconstructor** — NeRF/Gaussian-splatting successor for real-time scene capture. Design: 3D Gaussian splatting with relighting, compression, streaming reconstruction, editable scene graphs.

**5. 4D Dynamic Scene Model** — Reconstructs scenes through time (video → 4D). Design: dynamic Gaussians with deformation fields, occlusion-aware tracking, novel-view video synthesis.

**6. Tactile Perception Model** — Interprets touch sensor data for manipulation. Design: vision-based tactile sensors (GelSight-style) to contact maps, shear/force estimation, slip prediction.

**7. Olfactory Analyzer** — Maps e-nose sensor arrays to odor identities and intensities. Design: gas-sensor cross-sensitivity calibration, odor embedding space, drift compensation over sensor aging.

**8. Gustatory Analyzer** — Maps chemical sensors to taste profiles (sweet/salty/sour/bitter/umami). Design: electrochemical sensor fusion, concentration estimation, flavor compound correlation.

**9. Acoustic Scene Analyzer** — Understands environments from sound (indoor/outdoor/urban/nature). Design: sound event detection + spatial audio, source separation, acoustic scene classification with uncertainty.

**10. Ultra-Wideband Radar Interpreter** — Through-wall and short-range sensing via UWB. Design: impulse-response feature extraction, human presence/vital signs detection, clutter cancellation.

**11. LiDAR Semantic Segmenter** — Real-time point cloud understanding. Design: sparse voxel convolutions, range-view + point-view fusion, panoptic segmentation, streaming perception.

**12. Thermal Vision Model** — Analyzes long-wave infrared imagery. Design: thermal-to-visible translation, emissivity-aware segmentation, heat leak detection, night-time perception.

**13. Terahertz Imager** — THz security and materials imaging. Design: THz spectral fingerprinting, concealed object detection, non-ionizing scanning protocols.

**14. Quantum Sensor Data Interpreter** — Extracts signals from quantum sensors (magnetometers, gravimeters). Design: quantum noise modeling, single-shot readout classification, field-gradient inversion.

**15. Gravitational Wave Detector Analyzer** — Identifies events in interferometer strain data. Design: matched-filter surrogate models, glitch classification, real-time event candidate ranking.

**16. Neutrino Event Classifier** — Classifies neutrino interactions in detectors. Design: point-cloud networks over hits, flavor identification, energy reconstruction, background rejection.

**17. Subatomic Particle Track Reconstructor** — Reconstructs particle trajectories in detectors. Design: graph neural networks over hits, track finding + fitting, calorimeter shower regression.

**18. Biosensor Signal Decoder** — Interprets continuous biosensor streams (glucose, lactate, cortisol). Design: multi-analyte fusion, artifact rejection, drift correction, event detection.

**19. EEG Decoder** — Thought-to-text and mental state decoding. Design: subject-invariant contrastive pretraining, motor imagery classification, BCI spellers, artifact removal.

**20. EMG Gesture Translator** — Maps muscle signals to gestures/commands. Design: surface EMG feature extraction, temporal convolution, user calibration with few shots, prosthetic control.

**21. ECG Rhythm Analyzer** — Detects arrhythmias and cardiac events. Design: beat segmentation, rhythm classification (A-fib, flutter, VT), signal quality gating, explainable P-QRS-T attribution.

**22. Neural Spike Sorter** — Clusters extracellular spikes by neuron identity. Design: waveform embedding, online clustering, drift tracking, multi-electrode array coordination.

**23. Calcium Imaging Analyzer** — Extracts neuronal activity from fluorescence videos. Design: motion correction, cell detection, deconvolution of calcium transients, population activity decoding.

**24. MRI Denoiser & Super-Resolver** — Faster, clearer MRI from undersampled acquisitions. Design: physics-informed reconstruction (unrolled networks), motion artifact removal, multi-contrast transfer.

**25. Ultrasound Interpreter** — Real-time sonography analysis. Design: view classification, standard-plane detection, automated measurements, fetal biometry.

**26. X-Ray Dual-Energy Analyzer** — Material decomposition from dual-energy scans. Design: basis material decomposition, bone/soft-tissue separation, security screening support.

**27. Holographic Image Reconstructor** — Recovers focus and phase from holograms. Design: digital holography reconstruction, phase retrieval, autofocus, particle tracking in volumes.

**28. Speckle Pattern Analyzer** — Extracts motion/strain from laser speckle. Design: speckle correlation tracking, vibrometry, blood-flow imaging (LSCI), sub-pixel displacement.

**29. Hyperspectral Imager** — Analyzes hundreds of spectral bands per pixel. Design: spectral unmixing, material identification, vegetation stress detection, spectral super-resolution.

**30. Satellite Multi-Spectral Interpreter** — Land-use and change detection from orbit. Design: temporal stack analysis, cloud/shadow masking, semantic segmentation, change point detection.

**31. Atmospheric Sensor Fuser** — Merges weather station, radiosonde, and remote sensing. Design: 4D-variational-style assimilation, forecast nudging, observation quality weighting.

**32. Ocean Sonar Mapper** — Seafloor mapping and underwater object detection. Design: sidescan/multibeam fusion, bathymetry reconstruction, habitat classification.

**33. Seismic Interpreter** — Earthquake detection and subsurface imaging. Design: phase picking, magnitude estimation, arrival association, microseismic event location.

**34. Structural Vibration Analyzer** — Bridge/building health from accelerometer streams. Design: modal frequency tracking, damage index learning, ambient vibration separation, early-warning thresholds.

**35. Infrared Gas Imager** — Detects gas leaks in IR video. Design: gas plume segmentation, spectral absorption matching, leak rate estimation, false-alarm suppression.

**36. Millimeter-Wave Body Scanner Analyzer** — Security screening from mmWave reflections. Design: holographic reconstruction, anomaly localization, privacy-preserving human silhouettes.

**37. Chemical Spectrometer Interpreter** — Raman/FTIR/UV-Vis spectra to compounds. Design: spectral database retrieval, mixture decomposition, instrument transfer, concentration regression.

**38. Mass Spectrometry Analyzer** — Peaks to molecules/proteins. Design: peak picking, isotope pattern fitting, library search, de novo sequencing.

**39. Chromatography Peak Finder** — Resolves overlapping chromatographic peaks. Design: peak deconvolution, retention-time alignment, baseline correction, quality scoring.

**40. Flow Cytometry Classifier** — Cell populations from multi-channel scatter/fluorescence. Design: unsupervised gating, rare-cell detection, dimensionality reduction, batch effect correction.

**41. Microscopy Image Restorer** — Deblurs and denoises microscopy beyond optics limits. Design: supervised + self-supervised restoration, structured illumination reconstruction, super-resolution microscopy.

**42. Cryo-EM Map Interpreter** — Builds atomic models from density maps. Design: map sharpening, backbone tracing, side-chain fitting, confidence scoring per residue.

**43. X-Ray Crystallography Solver** — Structure determination from diffraction data. Design: phasing support, electron density interpretation, automated model building, twinning detection.

**44. Neutron Scattering Analyzer** — Interprets SANS/reflectometry data. Design: model fitting to scattering curves, structural parameter estimation, uncertainty propagation.

**45. Multisensory Alignment Model** — Learns cross-sensor correspondences without labels. Design: contrastive alignment across time-synced sensors, calibration-free fusion, sensor dropout robustness.

**46. Cross-Modal Translation Engine** — Converts between modalities (vision→haptics, audio→vibration). Design: generative translation with fidelity metrics, modality-agnostic latent space, real-time operation.

**47. Egocentric Perception Model** — First-person vision understanding (hand-object interaction). Design: headset/glasses camera streams, gaze-conditioned attention, action anticipation, privacy-preserving on-device processing.

**48. Peripheral Vision Simulator** — Predicts low-resolution peripheral perception. Design: foveation models, saliency-conditioned sampling, visual search in peripheral field, AR rendering optimization.

**49. Predictive Perception Model** — Anticipates what will be perceived next. Design: predictive coding stacks, motion extrapolation, occlusion reasoning, attention to prediction errors.

**50. Perception Calibration Auditor** — Audits perception models for bias, drift, and failure. Design: benchmark suites per sensor, failure attribution, calibration curves, deployment health checks.

---
*Generated 2026-08-01 · BrainBuilder Model Catalog · Domain 02 of 20*
