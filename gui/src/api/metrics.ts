export interface MetricPoint {
  epoch: number;
  step: number;
  loss: number;
  // True only on the final point of a run that stopped early because loss
  // hadn't improved for `patience` epochs — see core/src/runtime/standard_trainer.rs.
  stopped_early: boolean;
}
