# BrainBuilder Built-in Models

This directory contains local model weight files used by BrainBuilder and
shareable with other software you build.

## Layout

Each model should live in its own subdirectory, named after the model
identifier or a human-friendly slug, e.g.:

```
models/built-in/
  README.md
  models.json
  qwen/
    qwen2.5-0.5B-Instruct/
      config.json
      model.safetensors
      tokenizer.json
  mistral/
    mistral-7b-instruct/
      model.gguf
```

Supported formats are the same as the HuggingFace Hub cache:
- `.safetensors`
- `.gguf`
- `.onnx`
- `.bin` / `.pth` / `.pt`

## models.json

`models.json` is a simple manifest other tools can read without importing
BrainBuilder. It lists every model under this directory, its format(s), and
the files that make it up.

```json
[
  {
    "repo_id": "qwen/Qwen2.5-0.5B-Instruct",
    "path": "qwen/qwen2.5-0.5B-Instruct",
    "formats": ["safetensors"],
    "files": ["config.json", "model.safetensors", "tokenizer.json"]
  }
]
```

## Sharing

Because this is just a normal directory of files, you can:
- symlink it from another project,
- mount it in a container,
- add it as a Git LFS tracked directory,
- or point other tools directly at `models/built-in`.

The frontend Model Hub also scans this directory automatically, so any model
you drop here shows up without extra configuration.
