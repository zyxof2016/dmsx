use crate::types::*;

/// AI 引擎抽象——可对接 OpenAI/本地模型/规则引擎。
///
/// 生产实现需注入 LLM client（OpenAI/Ollama/vLLM）、
/// 向量检索（pgvector/Qdrant）、时序分析（ClickHouse SQL 或本地 ML）。
#[allow(async_fn_in_trait)]
pub trait AiEngine: Send + Sync + 'static {
    async fn detect_anomalies(
        &self,
        req: &AnomalyDetectionRequest,
    ) -> Result<Vec<AnomalyReport>, AiError>;

    async fn recommend_policies(
        &self,
        req: &PolicyRecommendationRequest,
    ) -> Result<Vec<PolicyRecommendation>, AiError>;

    async fn chat(&self, req: &AssistantChatRequest) -> Result<AssistantChatResponse, AiError>;

    async fn predict_maintenance(
        &self,
        req: &PredictionRequest,
    ) -> Result<Vec<PredictionReport>, AiError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("model unavailable: {0}")]
    ModelUnavailable(String),
    #[error("context too large: {0}")]
    ContextTooLarge(String),
    #[error("internal: {0}")]
    Internal(String),
}
