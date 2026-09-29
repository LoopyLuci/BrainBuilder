export interface MetricPoint {
  epoch: number;
  step: number;
  loss: number;
  // True only on the final point of a run that stopped early because loss
  // hadn't improved for `patience` epochs — see core/src/runtime/standard_trainer.rs.
  stopped_early: boolean;
  // The actual learning rate used for this point's step — equal to the
  // configured lr unless `lr_decay_epochs` is set, in which case it
  // periodically halves. See core/src/runtime/standard_trainer.rs.
  current_lr: number;
}
