use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// AI 功能模块统一输入上下文。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiContext {
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub locale: String,
}

// ---------------------------------------------------------------------------
// 异常检测
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyLevel {
    Normal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionRequest {
    pub ctx: AiContext,
    pub device_ids: Option<Vec<Uuid>>,
    pub time_range_hours: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyReport {
    pub id: Uuid,
    pub device_id: Uuid,
    pub level: AnomalyLevel,
    pub category: String,
    pub summary: String,
    pub details: serde_json::Value,
    pub suggested_actions: Vec<SuggestedAction>,
    pub detected_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// 策略推荐
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRecommendationRequest {
    pub ctx: AiContext,
    pub scope_description: Option<String>,
    pub objective: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRecommendation {
    pub id: Uuid,
    pub name: String,
    pub rationale: String,
    pub confidence: f64,
    pub spec_suggestion: serde_json::Value,
    pub rollout_suggestion: serde_json::Value,
    pub risk_notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// 自然语言助手
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantChatRequest {
    pub ctx: AiContext,
    pub messages: Vec<AssistantMessage>,
    pub intent_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantChatResponse {
    pub reply: String,
    pub actions: Vec<SuggestedAction>,
    pub references: Vec<AssistantReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantReference {
    pub resource_type: String,
    pub resource_id: String,
    pub label: String,
}

// ---------------------------------------------------------------------------
// 预测性维护
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRequest {
    pub ctx: AiContext,
    pub device_ids: Option<Vec<Uuid>>,
    pub horizon_days: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionReport {
    pub device_id: Uuid,
    pub risk_level: RiskLevel,
    pub predicted_issue: String,
    pub probability: f64,
    pub eta_days: Option<f64>,
    pub suggested_actions: Vec<SuggestedAction>,
}

// ---------------------------------------------------------------------------
// 共享
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: String,
    pub label: String,
    pub payload: serde_json::Value,
    pub auto_executable: bool,
}
