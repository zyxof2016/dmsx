use chrono::Utc;
use uuid::Uuid;

use crate::engine::{AiEngine, AiError};
use crate::types::*;

/// 基于规则 + 统计阈值的异常检测（Phase0 默认实现）。
/// Phase1+ 可替换为 ML 时序异常模型或 LLM 辅助分析。
pub struct RuleBasedAnomalyDetector;

impl AiEngine for RuleBasedAnomalyDetector {
    async fn detect_anomalies(
        &self,
        req: &AnomalyDetectionRequest,
    ) -> Result<Vec<AnomalyReport>, AiError> {
        let _hours = req.time_range_hours.unwrap_or(24);
        // TODO: 查询 ClickHouse 心跳/遥测，执行统计偏差检测
        //       - 心跳丢失 > 阈值 → 离线异常
        //       - CPU/内存突增 → 资源异常
        //       - 策略漂移 → 合规异常
        //       - 命令失败率飙升 → 执行异常
        Ok(vec![AnomalyReport {
            id: Uuid::new_v4(),
            device_id: req
                .device_ids
                .as_ref()
                .and_then(|v| v.first().copied())
                .unwrap_or(Uuid::nil()),
            level: AnomalyLevel::Normal,
            category: "heartbeat".into(),
            summary: "所有设备心跳正常".into(),
            details: serde_json::json!({}),
            suggested_actions: vec![],
            detected_at: Utc::now(),
        }])
    }

    async fn recommend_policies(
        &self,
        _req: &PolicyRecommendationRequest,
    ) -> Result<Vec<PolicyRecommendation>, AiError> {
        // TODO: 分析当前设备画像 + 合规发现 → 生成策略建议
        Ok(vec![])
    }

    async fn chat(&self, _req: &AssistantChatRequest) -> Result<AssistantChatResponse, AiError> {
        Err(AiError::ModelUnavailable(
            "本地规则引擎不支持自由对话，请配置 LLM 后端".into(),
        ))
    }

    async fn predict_maintenance(
        &self,
        _req: &PredictionRequest,
    ) -> Result<Vec<PredictionReport>, AiError> {
        // TODO: 基于心跳趋势 + 命令失败历史 → 预测故障概率
        Ok(vec![])
    }
}
