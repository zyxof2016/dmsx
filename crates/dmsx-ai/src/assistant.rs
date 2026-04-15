use crate::engine::{AiEngine, AiError};
use crate::types::*;

/// LLM 驱动的智能助手（对接 OpenAI 兼容 API / 本地 Ollama / vLLM）。
/// 系统提示词内嵌平台知识（设备模型、API 能力、策略语法），
/// 将用户自然语言映射为结构化操作意图。
pub struct LlmAssistant {
    pub api_base: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl LlmAssistant {
    pub fn new(api_base: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_base: api_base.into(),
            model: model.into(),
            api_key: None,
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

impl AiEngine for LlmAssistant {
    async fn detect_anomalies(
        &self,
        _req: &AnomalyDetectionRequest,
    ) -> Result<Vec<AnomalyReport>, AiError> {
        // LLM 模式下，异常检测仍走统计引擎，LLM 仅做摘要/归因
        Err(AiError::Internal(
            "LLM 助手不直接执行异常检测，请走 anomaly 模块".into(),
        ))
    }

    async fn recommend_policies(
        &self,
        _req: &PolicyRecommendationRequest,
    ) -> Result<Vec<PolicyRecommendation>, AiError> {
        // TODO: 构造 prompt（设备画像 + 目标 + 当前策略）→ LLM → 解析 JSON
        Err(AiError::ModelUnavailable("LLM 策略推荐尚未实现".into()))
    }

    async fn chat(&self, _req: &AssistantChatRequest) -> Result<AssistantChatResponse, AiError> {
        // TODO: POST {api_base}/v1/chat/completions
        //   system: 平台知识（设备类型、API 列表、策略语法、安全约束）
        //   + function_calling / tool_use 映射到内部 API
        //   → 解析 reply + actions
        Err(AiError::ModelUnavailable(format!(
            "LLM endpoint {} model {} 尚未对接",
            self.api_base, self.model
        )))
    }

    async fn predict_maintenance(
        &self,
        _req: &PredictionRequest,
    ) -> Result<Vec<PredictionReport>, AiError> {
        Err(AiError::ModelUnavailable("LLM 预测维护尚未实现".into()))
    }
}
