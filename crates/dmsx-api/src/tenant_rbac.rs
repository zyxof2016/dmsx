use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::dto::{
    RbacRole, TenantCustomRole, TenantRbacRolesUpsertReq, TenantRoleBinding,
    TenantRoleBindingsUpsertReq,
};

pub const TENANT_RBAC_ROLES_KEY: &str = "tenant_rbac_roles_v1";
pub const TENANT_ROLE_BINDINGS_KEY: &str = "tenant_role_bindings_v1";

pub const KNOWN_PERMISSION_NAMES: &[&str] = &[
    "platform.read",
    "platform.write",
    "stats.read",
    "stats.write",
    "devices.read",
    "devices.write",
    "policies.read",
    "policies.write",
    "commands.read",
    "commands.write",
    "device_shadow.read",
    "device_shadow.write",
    "artifacts.read",
    "artifacts.write",
    "compliance.read",
    "compliance.write",
    "remote_desktop.read",
    "remote_desktop.write",
    "ai_assist.read",
    "ai_assist.write",
    "generic_tenant_resource.read",
    "generic_tenant_resource.write",
    "tenant_rbac.read",
    "tenant_rbac.write",
];

const PLATFORM_ADMIN_PERMISSIONS: &[&str] = KNOWN_PERMISSION_NAMES;
const PLATFORM_VIEWER_PERMISSIONS: &[&str] = &[
    "platform.read",
];
const TENANT_ADMIN_PERMISSIONS: &[&str] = &[
    "stats.read",
    "stats.write",
    "devices.read",
    "devices.write",
    "policies.read",
    "policies.write",
    "commands.read",
    "commands.write",
    "device_shadow.read",
    "device_shadow.write",
    "artifacts.read",
    "artifacts.write",
    "compliance.read",
    "compliance.write",
    "remote_desktop.read",
    "remote_desktop.write",
    "ai_assist.read",
    "ai_assist.write",
    "generic_tenant_resource.read",
    "generic_tenant_resource.write",
    "tenant_rbac.read",
    "tenant_rbac.write",
];
const SITE_ADMIN_PERMISSIONS: &[&str] = &[
    "devices.read",
    "devices.write",
    "commands.read",
    "commands.write",
    "device_shadow.read",
    "device_shadow.write",
    "remote_desktop.read",
    "remote_desktop.write",
    "generic_tenant_resource.read",
    "generic_tenant_resource.write",
    "policies.read",
    "artifacts.read",
    "ai_assist.read",
    "compliance.read",
    "stats.read",
];
const OPERATOR_PERMISSIONS: &[&str] = &[
    "devices.read",
    "devices.write",
    "commands.read",
    "commands.write",
    "device_shadow.read",
    "device_shadow.write",
    "remote_desktop.read",
    "remote_desktop.write",
    "policies.read",
    "artifacts.read",
    "ai_assist.read",
    "compliance.read",
    "stats.read",
    "generic_tenant_resource.read",
];
const AUDITOR_PERMISSIONS: &[&str] = &[
    "stats.read",
    "devices.read",
    "policies.read",
    "commands.read",
    "device_shadow.read",
    "artifacts.read",
    "compliance.read",
    "generic_tenant_resource.read",
];
const READONLY_PERMISSIONS: &[&str] = &[
    "stats.read",
    "devices.read",
    "policies.read",
    "commands.read",
    "device_shadow.read",
    "artifacts.read",
    "compliance.read",
    "generic_tenant_resource.read",
];

pub fn builtin_role_permissions(name: &str) -> &'static [&'static str] {
    match name {
        "PlatformAdmin" => PLATFORM_ADMIN_PERMISSIONS,
        "PlatformViewer" => PLATFORM_VIEWER_PERMISSIONS,
        "TenantAdmin" => TENANT_ADMIN_PERMISSIONS,
        "SiteAdmin" => SITE_ADMIN_PERMISSIONS,
        "Operator" => OPERATOR_PERMISSIONS,
        "Auditor" => AUDITOR_PERMISSIONS,
        "ReadOnly" => READONLY_PERMISSIONS,
        _ => &[],
    }
}

pub fn custom_role_to_rbac_role(role: TenantCustomRole) -> RbacRole {
    RbacRole {
        name: role.name,
        scope: "tenant".to_string(),
        description: role.description,
        platform_read: false,
        platform_write: false,
        tenant_read: role.permissions.iter().any(|permission| permission.ends_with(".read")),
        tenant_write: role.permissions.iter().any(|permission| permission.ends_with(".write")),
        permissions: role.permissions,
        builtin: false,
    }
}

pub fn builtin_rbac_roles() -> Vec<RbacRole> {
    vec![
        RbacRole {
            name: "PlatformAdmin".to_string(),
            scope: "platform".to_string(),
            description: "平台级完全管理权限，可读写所有平台与租户资源。".to_string(),
            platform_read: true,
            platform_write: true,
            tenant_read: true,
            tenant_write: true,
            permissions: builtin_role_permissions("PlatformAdmin").iter().map(|item| (*item).to_string()).collect(),
            builtin: true,
        },
        RbacRole {
            name: "PlatformViewer".to_string(),
            scope: "platform".to_string(),
            description: "平台级只读权限，可查看平台配置、租户目录、全局审计与平台健康。".to_string(),
            platform_read: true,
            platform_write: false,
            tenant_read: false,
            tenant_write: false,
            permissions: builtin_role_permissions("PlatformViewer").iter().map(|item| (*item).to_string()).collect(),
            builtin: true,
        },
        RbacRole {
            name: "TenantAdmin".to_string(),
            scope: "tenant".to_string(),
            description: "租户级完全管理权限，可读写当前租户下的大多数资源。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: true,
            permissions: builtin_role_permissions("TenantAdmin").iter().map(|item| (*item).to_string()).collect(),
            builtin: true,
        },
        RbacRole {
            name: "SiteAdmin".to_string(),
            scope: "tenant".to_string(),
            description: "租户级运维管理角色，可管理设备、命令、影子与远程桌面；策略、制品、AI 仅只读。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: true,
            permissions: builtin_role_permissions("SiteAdmin").iter().map(|item| (*item).to_string()).collect(),
            builtin: true,
        },
        RbacRole {
            name: "Operator".to_string(),
            scope: "tenant".to_string(),
            description: "租户级操作员角色，可执行设备与命令相关操作；策略、制品、AI、合规、统计仅只读。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: true,
            permissions: builtin_role_permissions("Operator").iter().map(|item| (*item).to_string()).collect(),
            builtin: true,
        },
        RbacRole {
            name: "Auditor".to_string(),
            scope: "tenant".to_string(),
            description: "租户级审计角色，仅允许只读访问，不允许远程桌面与 AI。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: false,
            permissions: builtin_role_permissions("Auditor").iter().map(|item| (*item).to_string()).collect(),
            builtin: true,
        },
        RbacRole {
            name: "ReadOnly".to_string(),
            scope: "tenant".to_string(),
            description: "租户级只读角色，可查看常规租户资源，不允许写操作、远程桌面与 AI。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: false,
            permissions: builtin_role_permissions("ReadOnly").iter().map(|item| (*item).to_string()).collect(),
            builtin: true,
        },
    ]
}

pub fn is_builtin_role_name(name: &str) -> bool {
    builtin_rbac_roles().iter().any(|role| role.name == name)
}

pub fn normalize_custom_roles(req: TenantRbacRolesUpsertReq) -> Result<Vec<TenantCustomRole>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut roles = Vec::with_capacity(req.custom_roles.len());

    for role in req.custom_roles {
        let name = role.name.trim();
        if name.is_empty() {
            return Err("角色名不能为空".to_string());
        }
        if !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Err(format!("角色名 '{name}' 只能包含字母、数字、下划线或连字符"));
        }
        if is_builtin_role_name(name) {
            return Err(format!("角色名 '{name}' 与内置角色冲突"));
        }
        if !seen.insert(name.to_string()) {
            return Err(format!("角色名 '{name}' 重复"));
        }

        let mut permissions = role
            .permissions
            .into_iter()
            .map(|permission| permission.trim().to_string())
            .filter(|permission| !permission.is_empty())
            .collect::<Vec<_>>();
        permissions.sort();
        permissions.dedup();

        for permission in &permissions {
            if !KNOWN_PERMISSION_NAMES.iter().any(|known| known == permission) {
                return Err(format!("未知权限 '{permission}'"));
            }
        }

        roles.push(TenantCustomRole {
            name: name.to_string(),
            description: role.description.trim().to_string(),
            permissions,
        });
    }

    roles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(roles)
}

pub fn parse_custom_roles_value(value: &Value) -> Result<Vec<TenantCustomRole>, String> {
    let parsed = serde_json::from_value::<TenantRbacRolesUpsertReq>(value.clone())
        .map_err(|err| format!("解析租户 RBAC 配置失败: {err}"))?;
    normalize_custom_roles(parsed)
}

pub fn normalize_role_bindings(req: TenantRoleBindingsUpsertReq) -> Result<Vec<TenantRoleBinding>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut bindings = Vec::with_capacity(req.bindings.len());

    for binding in req.bindings {
        let subject = binding.subject.trim();
        if subject.is_empty() {
            return Err("subject 不能为空".to_string());
        }
        if !seen.insert(subject.to_string()) {
            return Err(format!("subject '{subject}' 重复"));
        }

        let mut roles = binding
            .roles
            .into_iter()
            .map(|role| role.trim().to_string())
            .filter(|role| !role.is_empty())
            .collect::<Vec<_>>();
        roles.sort();
        roles.dedup();

        bindings.push(TenantRoleBinding {
            subject: subject.to_string(),
            display_name: binding.display_name.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
            roles,
        });
    }

    bindings.sort_by(|a, b| a.subject.cmp(&b.subject));
    Ok(bindings)
}

pub fn parse_role_bindings_value(value: &Value) -> Result<Vec<TenantRoleBinding>, String> {
    let parsed = serde_json::from_value::<TenantRoleBindingsUpsertReq>(value.clone())
        .map_err(|err| format!("解析租户角色绑定失败: {err}"))?;
    normalize_role_bindings(parsed)
}

pub fn build_role_bindings_value(bindings: &[TenantRoleBinding]) -> Value {
    json!({ "bindings": bindings })
}

pub fn build_custom_roles_value(custom_roles: &[TenantCustomRole]) -> Value {
    json!({ "custom_roles": custom_roles })
}

pub async fn load_custom_roles_from_db(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TenantCustomRole>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT value FROM system_settings WHERE tenant_id = $1 AND key = $2",
    )
    .bind(tenant_id)
    .bind(TENANT_RBAC_ROLES_KEY)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(Vec::new());
    };

    let value: Value = row.try_get("value")?;
    Ok(parse_custom_roles_value(&value).unwrap_or_default())
}

pub async fn load_role_bindings_from_db(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TenantRoleBinding>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT value FROM system_settings WHERE tenant_id = $1 AND key = $2",
    )
    .bind(tenant_id)
    .bind(TENANT_ROLE_BINDINGS_KEY)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(Vec::new());
    };

    let value: Value = row.try_get("value")?;
    Ok(parse_role_bindings_value(&value).unwrap_or_default())
}

pub async fn load_custom_roles_from_conn(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<Option<(Vec<TenantCustomRole>, DateTime<Utc>)>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT value, updated_at FROM system_settings WHERE tenant_id = $1 AND key = $2",
    )
    .bind(tenant_id)
    .bind(TENANT_RBAC_ROLES_KEY)
    .fetch_optional(conn)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let value: Value = row.try_get("value")?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
    let roles = parse_custom_roles_value(&value).unwrap_or_default();
    Ok(Some((roles, updated_at)))
}

pub async fn load_role_bindings_from_conn(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<Option<(Vec<TenantRoleBinding>, DateTime<Utc>)>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT value, updated_at FROM system_settings WHERE tenant_id = $1 AND key = $2",
    )
    .bind(tenant_id)
    .bind(TENANT_ROLE_BINDINGS_KEY)
    .fetch_optional(conn)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let value: Value = row.try_get("value")?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
    let bindings = parse_role_bindings_value(&value).unwrap_or_default();
    Ok(Some((bindings, updated_at)))
}
