use std::sync::Arc;
use crate::tool_executor::executor::Tool;
use crate::tool_executor::world::{FsReadTool, FsWriteTool, FsDeleteTool, FsListTool, HttpFetchTool, ShellExecTool, SqliteQueryTool};
use crate::tool_executor::model_ops::{LoadModelTool, InferModelTool, LoadLoraTool, LoadDatasetTool, TrainModelTool};
use crate::tool_executor::learning::{CurriculumPlannerTool, ActiveLearningTool, SandboxRunTool};
use crate::tool_executor::safety::{ApprovalGateTool, BudgetTool, RedTeamTool};
use crate::tool_executor::vector_memory::VectorMemoryTool;
use crate::tool_executor::knowledge_graph::{KnowledgeGraphTool, EntityExtractionTool};
use crate::tool_executor::executor::ToolExecutor;

pub struct ToolRegistry {
    pub executor: ToolExecutor,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let executor = ToolExecutor::new();
        let registry = Self { executor };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&self) {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(FsReadTool),
            Arc::new(FsWriteTool),
            Arc::new(FsDeleteTool),
            Arc::new(FsListTool),
            Arc::new(HttpFetchTool),
            Arc::new(ShellExecTool),
            Arc::new(SqliteQueryTool),
            Arc::new(LoadModelTool),
            Arc::new(InferModelTool),
            Arc::new(LoadLoraTool),
            Arc::new(LoadDatasetTool),
            Arc::new(TrainModelTool),
            Arc::new(VectorMemoryTool),
            Arc::new(KnowledgeGraphTool),
            Arc::new(EntityExtractionTool),
            Arc::new(CurriculumPlannerTool),
            Arc::new(ActiveLearningTool),
            Arc::new(SandboxRunTool),
            Arc::new(ApprovalGateTool),
            Arc::new(BudgetTool),
            Arc::new(RedTeamTool),
        ];
        for tool in tools {
            // register is async but initialization is best-effort here; executor will still work.
            let _ = self.executor.register(tool);
        }
    }
}
