//! Complete JSON-schema definitions for every tool the Concierge can call.
//! These are sent to the LLM so it knows the exact parameter shapes.

use serde_json::{json, Value};

/// Return the full list of tool definitions in OpenAI function-calling format.
pub fn get_all_tools() -> Vec<Value> {
    vec![
        // ── Model registry ──────────────────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "search_models",
                "description": "Search the local model registry by keyword, modality or architecture family.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Free-text search term"},
                        "modality": {
                            "type": "string",
                            "enum": ["text", "image", "audio", "video", "multimodal", "any"],
                            "description": "Optional modality filter"
                        },
                        "arch": {"type": "string", "description": "Architecture family (llama, mistral, qwen, …)"}
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "import_model",
                "description": "Import a model from a local file path into the registry.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Absolute or relative path to the model file/dir"},
                        "format": {
                            "type": "string",
                            "enum": ["gguf", "onnx", "safetensors", "pytorch", "mlx", "auto"],
                            "description": "Explicit format; 'auto' tries to detect"
                        },
                        "name": {"type": "string", "description": "Optional display name for the imported model"}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_models",
                "description": "List all models currently registered in the local registry.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "description": "Max number of entries to return", "default": 50}
                    }
                }
            }
        }),

        // ── Canvas / graph manipulation ─────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "add_node",
                "description": "Add a node to the visual composition canvas.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "node_type": {
                            "type": "string",
                            "description": "Type of node: model, adapter, router, data, prompt, output, custom, …"
                        },
                        "model_id": {"type": "string", "description": "Registry model ID when node_type is model"},
                        "label": {"type": "string", "description": "Optional human-readable label"},
                        "position": {
                            "type": "object",
                            "properties": {
                                "x": {"type": "number"},
                                "y": {"type": "number"}
                            },
                            "description": "Canvas coordinates"
                        },
                        "config": {"type": "object", "description": "Node-specific configuration object"}
                    },
                    "required": ["node_type"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "connect_nodes",
                "description": "Create a directed connection between two node sockets on the canvas.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "source_node": {"type": "string", "description": "Source node ID"},
                        "source_socket": {"type": "string", "description": "Output socket name on source"},
                        "target_node": {"type": "string", "description": "Target node ID"},
                        "target_socket": {"type": "string", "description": "Input socket name on target"}
                    },
                    "required": ["source_node", "source_socket", "target_node", "target_socket"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "modify_graph",
                "description": "Perform a high-level graph mutation (delete, move, rename, group, ungroup, …).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["delete", "move", "rename", "group", "ungroup", "duplicate", "clear"],
                            "description": "Mutation to apply"
                        },
                        "target": {"type": "string", "description": "Node ID, group ID, or 'all'"},
                        "payload": {"type": "object", "description": "Action-specific extra data (e.g. new position, new name)"}
                    },
                    "required": ["action", "target"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "inspect_node",
                "description": "Return detailed runtime information about a canvas node (config, status, sockets, last output).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "node_id": {"type": "string", "description": "ID of the node to inspect"}
                    },
                    "required": ["node_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "execute_graph",
                "description": "Run the current canvas graph with the supplied inputs and return the outputs.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "inputs": {
                            "type": "object",
                            "description": "Map of input-node IDs (or names) to values"
                        },
                        "timeout_ms": {"type": "integer", "description": "Optional execution timeout in milliseconds"}
                    },
                    "required": ["inputs"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "compare_outputs",
                "description": "Run two nodes (or subgraphs) on the same input and compare their outputs.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "node_a": {"type": "string"},
                        "node_b": {"type": "string"},
                        "input": {"type": "string", "description": "Shared input payload (JSON string or plain text)"}
                    },
                    "required": ["node_a", "node_b", "input"]
                }
            }
        }),

        // ── Data & training ─────────────────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "create_dataset",
                "description": "Create a new dataset from one or more source files/URLs.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Dataset name"},
                        "sources": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of file paths or URLs"
                        },
                        "labels": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Optional label files or class names"
                        },
                        "format": {"type": "string", "description": "Expected format (jsonl, csv, parquet, …)"}
                    },
                    "required": ["name", "sources"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "run_training",
                "description": "Launch a training / fine-tuning job.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "base_model": {"type": "string", "description": "Registry ID or path of the base model"},
                        "dataset": {"type": "string", "description": "Dataset name or path"},
                        "recipe": {
                            "type": "string",
                            "description": "Training recipe / hyper-parameter preset (lora, qlora, full, …)"
                        },
                        "output_name": {"type": "string", "description": "Name for the resulting adapter / model"},
                        "epochs": {"type": "integer"},
                        "learning_rate": {"type": "number"}
                    },
                    "required": ["base_model", "dataset", "recipe"]
                }
            }
        }),

        // ── Knowledge modules & merging ─────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "create_knowledge_module",
                "description": "Package a trained adapter (or other artefact) into a reusable Knowledge Module.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "base_model": {"type": "string"},
                        "adapter_path": {"type": "string"},
                        "metadata": {
                            "type": "object",
                            "description": "Arbitrary metadata (domain, version, author, …)"
                        },
                        "name": {"type": "string"}
                    },
                    "required": ["base_model", "adapter_path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "merge_models",
                "description": "Merge two or more models / adapters using a chosen strategy.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "model_ids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of registry IDs to merge"
                        },
                        "strategy": {
                            "type": "string",
                            "enum": ["linear", "ties", "dare", "slerp", "task_arithmetic"],
                            "description": "Merge algorithm"
                        },
                        "weights": {
                            "type": "array",
                            "items": {"type": "number"},
                            "description": "Optional per-model weights"
                        },
                        "output_name": {"type": "string"}
                    },
                    "required": ["model_ids", "strategy"]
                }
            }
        }),

        // ── Documentation & plugins ─────────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "search_docs",
                "description": "Search the OmniForge documentation and knowledge base.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "limit": {"type": "integer", "default": 5}
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_plugin",
                "description": "Generate a plugin or custom node implementation from a natural-language description.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {"type": "string", "description": "What the plugin should do"},
                        "language": {
                            "type": "string",
                            "enum": ["python", "rust", "typescript"],
                            "default": "python"
                        },
                        "name": {"type": "string", "description": "Desired plugin name"}
                    },
                    "required": ["description"]
                }
            }
        }),

        // ── Utility / meta ──────────────────────────────────────────────
        json!({
            "type": "function",
            "function": {
                "name": "get_platform_status",
                "description": "Return a high-level health / status snapshot of the OmniForge platform.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "set_code_agent_mode",
                "description": "Switch the code-generation backend between OpenCode Go (fast) and OpenCode Zen (deep).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["go", "zen"]
                        }
                    },
                    "required": ["mode"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "execute_python",
                "description": "Generate Python code for a task (via OpenCode) and, if the user has previously approved, execute it in the sandbox. Prefer write_plugin + explicit user confirmation for new code.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task": {"type": "string", "description": "Natural-language description of the code to generate"},
                        "code": {"type": "string", "description": "If supplied, execute this code instead of generating"},
                        "mode": {
                            "type": "string",
                            "enum": ["go", "zen"],
                            "description": "Code-agent mode for generation"
                        }
                    }
                }
            }
        }),

        json!({
            "type": "function",
            "function": {
                "name": "set_model",
                "description": "Switch the active LLM model (OpenRouter free models, Ollama tags, or any compatible id).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "model": {"type": "string", "description": "Model identifier, e.g. mistralai/mistral-7b-instruct:free"}
                    },
                    "required": ["model"]
                }
            }
        }),
    ]
}