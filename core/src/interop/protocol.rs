use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrainBuilderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Component not found: {0}")]
    ComponentNotFound(String),
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    #[error("Shape inference failed: {0}")]
    ShapeInference(String),
    #[error("Python error: {0}")]
    Python(String),
    #[error("Racket error: {0}")]
    Racket(String),
    #[error("Clojure error: {0}")]
    Clojure(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Graph error: {0}")]
    GraphError(String),
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("Hardware error: {0}")]
    Hardware(String),
}
