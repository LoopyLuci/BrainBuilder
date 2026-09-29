# Domain 08 — Climate & Environment

*Architecture substrate: earth-system emulators (physics-informed ML surrogates of GCMs) with uncertainty quantification, extreme-event tail modeling, and decision-oriented outputs for mitigation/adaptation.*

**1. Earth System Digital Twin** — Continuous virtual Earth for scenario testing. Design: coupled atmosphere/ocean/land/ice emulation, observation assimilation, policy scenario replay.

**2. Global Climate Model (high-res)** — Kilometer-scale climate simulation. Design: ML-accelerated physics parameterization, hybrid dynamical-statistical downscaling, energy-consistent budgets.

**3. Regional Downscaler** — Localizes global projections. Design: statistical + dynamical downscaling, bias correction, extremes preservation, city-scale outputs.

**4. Extreme Weather Forecaster** — Predicts heat/cold/wind extremes. Design: extreme value theory coupling, ensemble tails, return-period estimation, attribution support.

**5. Hurricane Track Predictor** — Forecasts tropical cyclone paths/intensity. Design: multi-model consensus, rapid intensification prediction, storm-surge coupling, uncertainty cones.

**6. Flood Inundation Mapper** — Real-time flood extent forecasting. Design: hydrologic-hydrodynamic coupling, DEM-based routing, satellite validation, urban drainage modeling.

**7. Wildfire Spread Modeler** — Predicts fire behavior. Design: fuel-moisture dynamics, wind-terrain interaction, ember transport, containment planning, smoke dispersion.

**8. Heatwave Intensity Predictor** — Heat events with health impact. Design: urban heat island modeling, wet-bulb temperature forecasting, vulnerability mapping, warning triggers.

**9. Drought Early Warning** — Anticipates drought onset/severity. Design: soil-moisture/vegetation anomaly tracking, teleconnection drivers, agricultural/water-supply impact, recovery prediction.

**10. Sea Level Rise Projector** — Localized sea-level scenarios. Design: ice-sheet dynamics coupling, land motion (GIA/subsidence), storm-surge compounding, adaptation planning.

**11. Ocean Current Modeler** — Ocean circulation and heat transport. Design: eddy-resolving emulation, overturning circulation monitoring, marine heatwave prediction.

**12. Coral Reef Health Monitor** — Reef condition from imagery/environment. Design: bleaching detection, recovery dynamics, stressor attribution, restoration prioritization.

**13. Biodiversity Assessor** — Measures ecosystem status. Design: eDNA + acoustic + camera fusion, species occupancy modeling, functional diversity metrics.

**14. Species Extinction Risk Model** — IUCN-style risk assessment. Design: population trend modeling, threat mapping, Red List criteria automation, climate exposure.

**15. Habitat Connectivity Planner** — Plans wildlife corridors. Design: resistance-surface modeling, graph-based connectivity, barrier mitigation, climate-driven range shift planning.

**16. Poacher Detection Model** — Detects illegal activity in protected areas. Design: acoustic/sensor/camera fusion, patrol route optimization, deterrence assessment.

**17. Wildlife Trafficking Interdictor** — Disrupts illegal trade networks. Design: shipment anomaly detection, market monitoring, network analysis, enforcement targeting.

**18. Invasive Species Early Warner** — Early detection of invasions. Design: propagule pressure modeling, niche matching, citizen-science data fusion, rapid-response triggers.

**19. Pollinator Population Tracker** — Monitors pollinator health. Design: hive sensor fusion, forage availability mapping, pesticide exposure modeling, decline attribution.

**20. Forest Carbon Stocktaker** — Quantifies forest carbon. Design: LiDAR/satellite fusion, allometric modeling, disturbance accounting, verification-grade estimates.

**21. Deforestation Alerter** — Near-real-time forest loss detection. Design: change detection from SAR/optical, alert prioritization, supply-chain deforestation risk.

**22. Soil Health Analyzer** — Assesses soil condition at scale. Design: spectroscopy + sensor fusion, carbon/nutrient estimation, degradation risk, management recommendations.

**23. Permafrost Thaw Modeler** — Projects thaw and its feedbacks. Design: thermal-hydrological coupling, carbon release estimation, infrastructure risk, methane hotspots.

**24. Methane Leak Detector** — Finds methane emissions. Design: satellite plume detection, source attribution, emission quantification, repair prioritization.

**25. CO2 Flux Estimator** — Gridded carbon flux accounting. Design: inverse modeling, OCO-2/3 + in-situ fusion, biome attribution, national inventory reconciliation.

**26. Carbon Offset Verifier** — Validates offset claims. Design: baseline modeling, additionality assessment, leakage accounting, permanence risk, fraud detection.

**27. Renewable Resource Assessor** — Maps solar/wind/geothermal potential. Design: resource climatology, techno-economic screening, land-use conflict checks, grid proximity.

**28. Solar Farm Yield Forecaster** — Predicts PV output. Design: irradiance ensemble forecasting, soiling/degradation modeling, curtailment optimization, revenue forecasting.

**29. Wind Farm Siting Optimizer** — Optimizes turbine placement. Design: wake modeling, layout search, noise/visual constraints, energy yield vs cost.

**30. Grid Integration Planner** — Integrates variable renewables. Design: capacity expansion with variability, storage sizing, transmission bottlenecks, flexibility requirements.

**31. Climate Adaptation Advisor** — Recommends adaptation actions. Design: exposure/vulnerability mapping, option appraisal, co-benefit accounting, maladaptation screening.

**32. Climate Migration Modeler** — Projects climate-driven movement. Design: push/pull factor modeling, destination pressure, policy scenarios, humanitarian need.

**33. Climate Conflict Predictor** — Assesses climate-security risks. Design: resource stress indicators, conflict correlation modeling, early-warning, peacebuilding sensitivity.

**34. Geoengineering Impact Modeler** — Simulates intervention consequences. Design: solar-radiation-management and CDR simulation, regional climate response, termination shock risk.

**35. Solar Radiation Management Assessor** — Evaluates SRM options. Design: injection-scenario simulation, side-effect quantification, governance framing, monitoring needs.

**36. Ocean Acidification Monitor** — Tracks ocean chemistry change. Design: carbonate-system modeling, sensor network assimilation, ecosystem vulnerability, mitigation tracking.

**37. Plastic Pollution Tracker** — Models plastic flows. Design: riverine input modeling, ocean accumulation zones, microplastic transport, cleanup effectiveness.

**38. Microplastic Source Mapper** — Attributes microplastic sources. Design: polymer fingerprinting, source apportionment, transport modeling, exposure assessment.

**39. Air Quality Forecaster** — Predicts pollution episodes. Design: emission + meteorological coupling, chemical transport emulation, health alerting, source attribution.

**40. Urban Heat Island Reducer** — Designs cooling interventions. Design: surface-energy modeling, green infrastructure optimization, cool-roof/pavement scenarios, equity-aware benefits.

**41. Green Roof Planner** — Optimizes rooftop greening. Design: stormwater retention modeling, thermal benefit, structural load constraints, cost-benefit.

**42. Water Quality Monitor** — Assesses water body health. Design: satellite + in-situ fusion, harmful algal bloom prediction, pathogen indicators, source tracking.

**43. Watershed Manager** — Balances competing water needs. Design: hydrologic budget modeling, allocation optimization, environmental flow protection, drought operations.

**44. Aquifer Recharge Planner** — Plans managed recharge. Design: infiltration feasibility, water-budget accounting, water-quality interaction, long-term sustainability.

**45. Desalination Optimizer** — Improves desalination efficiency. Design: process modeling (RO/thermal), energy recovery, brine management, cost minimization.

**46. Wetland Restoration Planner** — Designs wetland restoration. Design: hydrology restoration modeling, vegetation succession, carbon/methane tradeoffs, wildlife benefit.

**47. Blue Carbon Accountant** — Accounts for coastal carbon. Design: mangrove/seagrass/saltmarsh carbon stocks, emissions factors, project crediting, MRV support.

**48. Climate Finance Allocator** — Directs adaptation/mitigation capital. Design: risk-return modeling, project pipeline scoring, co-financing leverage, impact measurement.

**49. ESG Impact Measurer** — Quantifies sustainability performance. Design: materiality mapping, data quality scoring, greenwashing detection, benchmark-relative assessment.

**50. Net-Zero Pathway Planner** — Designs credible decarbonization routes. Design: sectoral abatement curves, technology diffusion modeling, policy sensitivity, residual-emissions management.

---
*Generated 2026-08-01 · BrainBuilder Model Catalog · Domain 08 of 20*
