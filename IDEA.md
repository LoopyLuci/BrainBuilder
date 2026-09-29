BrainBuilder is a desktop application for visually building, training, predicting
with, and extending neural network graphs — backed by a real Rust orchestration engine
and a real PyTorch training backend, not a simulation or a mockup. You drag components
onto a canvas, wire them together, point at a dataset, and train — with a live loss
chart, checkpoint management, prediction, and interpretability tooling built in. Where
BrainBuilder goes further than a typical graph editor is that the library of
components, the graphs themselves, and even the app's own codebase are all things the
app can extend: describe a model in plain English and get a trained graph, describe a
new layer and get a real, sandboxed, installable component, or hand the app a coding
task and let a self-building agent implement it inside an isolated, test-gated git
