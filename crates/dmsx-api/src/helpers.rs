use dmsx_core::CommandStatus;

pub fn compute_shadow_delta(
    desired: &serde_json::Value,
    reported: &serde_json::Value,
) -> serde_json::Value {
    let mut delta = serde_json::Map::new();
    if let Some(desired_obj) = desired.as_object() {
        let reported_obj = reported.as_object();
        for (key, value) in desired_obj {
            let differs = reported_obj.map_or(true, |reported| reported.get(key) != Some(value));
            if differs {
                delta.insert(key.clone(), value.clone());
            }
        }
    }
    serde_json::Value::Object(delta)
}

pub fn command_status_from_exit_code(exit_code: Option<i32>) -> CommandStatus {
    if exit_code.unwrap_or(-1) == 0 {
        CommandStatus::Succeeded
    } else {
        CommandStatus::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::{command_status_from_exit_code, compute_shadow_delta};
    use dmsx_core::CommandStatus;

    #[test]
    fn compute_shadow_delta_returns_only_changed_fields() {
        let desired = serde_json::json!({
            "hostname": "device-a",
            "agent_version": "1.2.3",
            "online_state": "online"
        });
        let reported = serde_json::json!({
            "hostname": "device-a",
            "agent_version": "1.0.0"
        });

        let delta = compute_shadow_delta(&desired, &reported);

        assert_eq!(
            delta,
            serde_json::json!({
                "agent_version": "1.2.3",
                "online_state": "online"
            })
        );
    }

    #[test]
    fn compute_shadow_delta_returns_empty_object_for_non_object_desired() {
        let delta = compute_shadow_delta(&serde_json::json!(null), &serde_json::json!({}));
        assert_eq!(delta, serde_json::json!({}));
    }

    #[test]
    fn command_status_from_exit_code_maps_zero_to_succeeded() {
        assert_eq!(
            command_status_from_exit_code(Some(0)),
            CommandStatus::Succeeded
        );
    }

    #[test]
    fn command_status_from_exit_code_maps_non_zero_and_none_to_failed() {
        assert_eq!(
            command_status_from_exit_code(Some(1)),
            CommandStatus::Failed
        );
        assert_eq!(command_status_from_exit_code(None), CommandStatus::Failed);
    }
}
