# Domain 12 — Robotics & Physical AI

*Architecture substrate: embodied foundation models (vision-language-action) with physics-aware world models, sim-to-real transfer, safety envelopes, and hierarchical control. Deployed on BrainBuilder's Rust/Tauri stack with real-time guarantees.*

**1. Whole-Body Motion Planner** — Full-body trajectory planning. Design: kinodynamic planning, contact sequencing, collision-aware optimization, task-space constraints.

**2. Dexterous Hand Controller** — Multi-finger manipulation. Design: tactile-guided grasping, in-hand reorientation, contact-rich policies, sim-to-real.

**3. Tactile Manipulation Model** — Touch-guided manipulation. Design: tactile sensing fusion, slip prevention, insertion skills, deformable-object handling.

**4. Locomotion Gait Designer** — Adaptive gaits. Design: gait libraries + RL, terrain adaptation, energy optimization, fault tolerance.

**5. Quadruped Balance Controller** — Rough-terrain quadruped control. Design: model-predictive control with ML dynamics, disturbance rejection, stair climbing, fall recovery.

**6. Humanoid Whole-Body Controller** — Bipedal humanoid control. Design: momentum-based balancing, whole-body MPC, dynamic walking, manipulation during motion.

**7. Bipedal Gait Stabilizer** — Push recovery and balance. Design: capture-point control, step adjustment, ankle/hip strategies, uneven terrain.

**8. Swarm Coordination Engine** — Decentralized robot teams. Design: flocking/consensus, task allocation, collision avoidance, communication constraints, resilience.

**9. Multi-Robot Task Allocator** — Optimal task distribution. Design: market-based allocation, temporal constraints, energy awareness, replanning.

**10. Warehouse Orchestrator** — Coordinated fulfillment. Design: order batching, pick-path optimization, fleet scheduling, congestion management, throughput forecasting.

**11. Autonomous Forklift Controller** — Pallet handling autonomy. Design: pallet detection, precision docking, load stability, safety zones, traffic rules.

**12. Last-Mile Delivery Robot Planner** — Sidewalk delivery autonomy. Design: pedestrian-aware planning, curb negotiation, doorstep detection, weather robustness.

**13. Drone Delivery Scheduler** — Air delivery logistics. Design: route optimization, battery/range constraints, airspace deconfliction, landing logistics.

**14. Aerial Inspection Planner** — Autonomous infrastructure checks. Design: coverage planning, defect detection, GPS-denied navigation, anomaly reporting.

**15. Underwater Robot Controller** — Subsea operations. Design: current compensation, acoustic positioning, imaging sonar fusion, manipulator control.

**16. Deep-Sea Explorer** — Autonomous ocean exploration. Design: adaptive sampling, habitat mapping, vehicle health autonomy, scientific target prioritization.

**17. Mine Survey Robot** — Underground mapping. Design: GPS-denied SLAM, gas safety, roof-void detection, mine layout reconstruction.

**18. Sewer Inspection Robot** — Pipe condition assessment. Design: pipe-centerline following, defect classification, flow handling, structural scoring.

**19. Pipe Crawler Controller** — In-pipe navigation. Design: diameter adaptation, bend traversal, tether management, cleaning/jointing tasks.

**20. Wall-Climbing Robot Planner** — Vertical surface mobility. Design: adhesion modeling (vacuum/magnetic), transition planning, payload stability, surface assessment.

**21. Soft Robot Actuator Model** — Soft-body actuation. Design: continuum kinematics, pneumatic modeling, deformable simulation, control mapping.

**22. Origami Robot Designer** — Foldable robot design. Design: crease-pattern search, deployment simulation, stiffness tuning, fabrication constraints.

**23. Tensegrity Robot Controller** — Tensegrity structure control. Design: cable actuation planning, shape change, rolling locomotion, robustness.

**24. Modular Reconfigurable Robot Planner** — Self-reconfiguring robots. Design: morphology search, docking/undocking planning, task-morphology matching, power routing.

**25. Human-Robot Interaction Model** — Safe natural interaction. Design: intent prediction, proxemics, gaze/gesture understanding, legible motion.

**26. Social Robot Dialogue Manager** — Conversational service robots. Design: multi-turn dialogue with grounding, turn-taking, emotional tone, memory of users.

**27. Collaborative Robot Safety Model** — Human-robot collaboration safety. Design: speed-separation monitoring, force limiting, risk assessment, certification support.

**28. Cobot Task Planner** — Shared task planning. Design: human-role modeling, handoff optimization, contingency handling, learned preferences.

**29. Robot Learning from Demonstration** — Teach-by-showing. Design: keyframe extraction, skill segmentation, generalization, refinement loops.

**30. Imitation Learning Engine** — Policy learning from expert data. Design: behavior cloning + DAgger, multimodal action distributions, safe exploration.

**31. Inverse Reinforcement Learning for Robots** — Reward inference. Design: reward decomposition, suboptimal-expert handling, transferable objectives, constraint recovery.

**32. Sim-to-Real Transfer Engine** — Sim-trained policies to hardware. Design: domain randomization, dynamics randomization, system identification, zero-shot transfer.

**33. Domain Randomization Optimizer** — Automated randomization. Design: distribution search, coverage metrics, transfer validation, sample efficiency.

**34. Physics Simulator for Robots** — High-fidelity training environments. Design: contact-rich simulation, soft-body support, sensor simulation, parallelism.

**35. Contact-Rich Manipulation Planner** — Pushing/grasping/assembly. Design: quasi-static models, hybrid force/motion control, contact graphs, robust execution.

**36. Peg-in-Hole Master** — Precision insertion. Design: search strategies, force/torque feedback, chamfer exploitation, uncertainty handling.

**37. Cable Routing Planner** — Wire/harness manipulation. Design: deformable modeling, grasp sequencing, tautness control, connector alignment.

**38. Cloth Manipulation Model** — Fabric handling. Design: cloth state estimation, flatten/fold primitives, wrinkle dynamics, vision-based tracking.

**39. Food Handling Robot** — Perishable item manipulation. Design: deformable/irregular objects, hygiene constraints, speed, portioning accuracy.

**40. Surgical Micro-Robot Controller** — Minimally invasive robotics. Design: tremor cancellation, haptic feedback, constrained motion, tool-tissue interaction.

**41. Nanorobot Swarm Coordinator** — Micro-scale agent coordination. Design: magnetic field steering, swarm signaling, targeted delivery, imaging feedback.

**42. Exoskeleton Gait Symbiosis Model** — Human-exoskeleton cooperation. Design: intention estimation, torque assistance, gait-phase detection, metabolic optimization.

**43. Prosthetic Intention Decoder** — Intuitive prosthetic intent. Design: residual muscle decoding, adaptive calibration, mode prediction, sensory feedback.

**44. Teleoperation Latency Compensator** — Smooth remote control over delay. Design: predictive display, input prediction, event-based control, haptic feedback over network.

**45. Haptic Feedback Synthesizer** — Touch rendering. Design: contact modeling, texture rendering, force feedback stability, perception alignment.

**46. Robot Self-Repair Planner** — Autonomous maintenance. Design: fault diagnosis, repair procedure planning, spare management, degraded-mode operation.

**47. Robot Recharging Scheduler** — Energy management. Design: battery forecasting, opportunistic charging, task-energy tradeoffs, fleet coordination.

**48. Fleet Learning Aggregator** — Shared learning across robots. Design: federated skill updates, experience replay sharing, policy distillation, safety screening.

**49. Robot Safety Certifier** — Safety assurance pipeline. Design: formal verification, runtime monitoring, safety-case generation, certification evidence.

**50. Embodied Intelligence Benchmarker** — Standardized robot evaluation. Design: task suites, sim benchmarks, hardware comparability, progress tracking.

---
*Generated 2026-08-01 · BrainBuilder Model Catalog · Domain 12 of 20*
