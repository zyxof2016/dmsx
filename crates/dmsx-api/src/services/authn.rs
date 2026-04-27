use std::collections::HashMap;

use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::auth::{AuthConfig, AuthMode};
use crate::auth_tokens::{
    issue_login_token, issue_login_transaction_token, verify_login_transaction_token,
};
use crate::dto::{
    LoginDecision, LoginDecisionKind, LoginReq, LoginResp, LoginTenantOption, LogoutReq,
    SelectLoginTenantReq,
};
use crate::error::map_db_error;
use crate::repo::control_accounts;
use crate::services::ServiceResult;
use crate::state::AppState;

const DEV_DEFAULT_TENANT_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000001);
const DEV_SECOND_TENANT_ID: Uuid = Uuid::from_u128(0x22222222222222222222222222222222);

pub async fn login(st: &AppState, req: &LoginReq) -> ServiceResult<LoginResp> {
    req.validate()?;
    ensure_login_available(&st.auth)?;

    let mut conn = st.db.acquire().await.map_err(map_db_error)?;
    let account = control_accounts::find_by_username(&mut conn, req.username())
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| dmsx_core::DmsxError::Unauthorized("用户名或密码错误".into()))?;

    if !account.is_active {
        return Err(dmsx_core::DmsxError::Forbidden("账号已停用".into()));
    }
    if hash_password(req.password()) != account.password_hash {
        return Err(dmsx_core::DmsxError::Unauthorized(
            "用户名或密码错误".into(),
        ));
    }

    let platform_roles = parse_roles(&account.platform_roles)?;
    let tenant_rows = control_accounts::list_tenants_for_account(&mut conn, account.id)
        .await
        .map_err(map_db_error)?;
    let tenant_options = tenant_rows
        .iter()
        .map(|row| LoginTenantOption {
            tenant_id: row.tenant_id,
            roles: parse_roles(&row.roles).unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    let has_platform = platform_roles
        .iter()
        .any(|role| matches!(role.as_str(), "PlatformAdmin" | "PlatformViewer"));
    let preferred_tenant_id = account
        .last_tenant_id
        .filter(|tid| tenant_options.iter().any(|option| option.tenant_id == *tid))
        .or(account.default_tenant_id)
        .or_else(|| tenant_options.first().map(|option| option.tenant_id));

    let decision = if has_platform && !tenant_options.is_empty() {
        LoginDecision {
            kind: LoginDecisionKind::ChooseScope,
            preferred_tenant_id,
            tenant_options,
        }
    } else if has_platform {
        LoginDecision {
            kind: LoginDecisionKind::PlatformOnly,
            preferred_tenant_id: None,
            tenant_options: Vec::new(),
        }
    } else if tenant_options.len() > 1 {
        LoginDecision {
            kind: LoginDecisionKind::ChooseTenant,
            preferred_tenant_id,
            tenant_options,
        }
    } else if tenant_options.len() == 1 {
        LoginDecision {
            kind: LoginDecisionKind::TenantOnly,
            preferred_tenant_id,
            tenant_options,
        }
    } else {
        return Err(dmsx_core::DmsxError::Forbidden(
            "账号未配置任何平台或租户权限".into(),
        ));
    };

    let mut available_scopes = Vec::new();
    if has_platform {
        available_scopes.push("platform".to_string());
    }
    if !decision.tenant_options.is_empty() || matches!(decision.kind, LoginDecisionKind::TenantOnly)
    {
        available_scopes.push("tenant".to_string());
    }

    let login_transaction_token =
        issue_login_transaction_token(&st.auth, account.id, &account.username)?;

    Ok(LoginResp {
        account_id: account.id,
        username: account.username,
        display_name: account.display_name,
        platform_roles,
        available_scopes,
        decision,
        login_transaction_token: Some(login_transaction_token),
        token: None,
        active_scope: None,
        active_tenant_id: None,
    })
}

pub async fn select_login_scope(
    st: &AppState,
    req: &SelectLoginTenantReq,
) -> ServiceResult<LoginResp> {
    req.validate()?;
    ensure_login_available(&st.auth)?;

    let mut conn = st.db.acquire().await.map_err(map_db_error)?;
    let account = control_accounts::find_by_username(&mut conn, req.username())
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| dmsx_core::DmsxError::Unauthorized("账号不存在".into()))?;
    if !account.is_active {
        return Err(dmsx_core::DmsxError::Forbidden("账号已停用".into()));
    }
    let token_account_id = verify_login_transaction_token(
        &st.auth,
        req.login_transaction_token.trim(),
        req.username(),
    )?;
    if token_account_id != account.id {
        return Err(dmsx_core::DmsxError::Unauthorized(
            "登录选择凭证与账号不匹配".into(),
        ));
    }

    let platform_roles = parse_roles(&account.platform_roles)?;
    let tenant_rows = control_accounts::list_tenants_for_account(&mut conn, account.id)
        .await
        .map_err(map_db_error)?;
    let tenant_roles_map = tenant_rows
        .iter()
        .map(|row| Ok((row.tenant_id, parse_roles(&row.roles)?)))
        .collect::<Result<HashMap<_, _>, dmsx_core::DmsxError>>()?;

    let selected_tenant = if req.scope == "tenant" {
        let tid = req
            .tenant_id
            .ok_or_else(|| dmsx_core::DmsxError::Validation("租户模式必须选择 tenant_id".into()))?;
        let roles = tenant_roles_map
            .get(&tid)
            .cloned()
            .ok_or_else(|| dmsx_core::DmsxError::Forbidden("当前账号无该租户权限".into()))?;
        control_accounts::touch_last_tenant(&mut conn, account.id, tid)
            .await
            .map_err(map_db_error)?;
        (tid, roles)
    } else {
        if !platform_roles
            .iter()
            .any(|role| matches!(role.as_str(), "PlatformAdmin" | "PlatformViewer"))
        {
            return Err(dmsx_core::DmsxError::Forbidden("当前账号无平台权限".into()));
        }
        (
            req.tenant_id
                .or(account.last_tenant_id)
                .or(account.default_tenant_id)
                .or_else(|| tenant_roles_map.keys().next().copied())
                .unwrap_or(Uuid::nil()),
            Vec::new(),
        )
    };

    let (active_tenant_id, active_tenant_roles) = selected_tenant;
    let allowed_tenant_ids = tenant_roles_map
        .keys()
        .copied()
        .filter(|tid| *tid != active_tenant_id)
        .collect::<Vec<_>>();
    let tenant_roles = tenant_roles_map
        .into_iter()
        .filter(|(_, roles)| !roles.is_empty())
        .collect::<HashMap<_, _>>();

    let token = issue_login_token(
        &st.auth,
        &account.username,
        active_tenant_id,
        allowed_tenant_ids,
        platform_roles.clone(),
        tenant_roles,
    )?;

    let mut available_scopes = Vec::new();
    if platform_roles
        .iter()
        .any(|role| matches!(role.as_str(), "PlatformAdmin" | "PlatformViewer"))
    {
        available_scopes.push("platform".to_string());
    }
    if !tenant_rows.is_empty() {
        available_scopes.push("tenant".to_string());
    }

    Ok(LoginResp {
        account_id: account.id,
        username: account.username,
        display_name: account.display_name,
        platform_roles: if req.scope == "tenant" {
            active_tenant_roles
        } else {
            platform_roles
        },
        available_scopes,
        decision: LoginDecision {
            kind: if req.scope == "platform" {
                LoginDecisionKind::PlatformOnly
            } else {
                LoginDecisionKind::TenantOnly
            },
            preferred_tenant_id: Some(active_tenant_id),
            tenant_options: Vec::new(),
        },
        login_transaction_token: None,
        token: Some(token),
        active_scope: Some(req.scope.clone()),
        active_tenant_id: Some(active_tenant_id),
    })
}

pub async fn logout(st: &AppState, ctx: &AuthContext, req: &LogoutReq) -> ServiceResult<()> {
    let mut conn = st.db.acquire().await.map_err(map_db_error)?;
    let account = control_accounts::find_by_username(&mut conn, &ctx.subject)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| dmsx_core::DmsxError::Unauthorized("账号不存在".into()))?;

    let tenant_rows = control_accounts::list_tenants_for_account(&mut conn, account.id)
        .await
        .map_err(map_db_error)?;
    let target_tenant_id = req.tenant_id.unwrap_or(ctx.tenant_id);

    if tenant_rows
        .iter()
        .any(|row| row.tenant_id == target_tenant_id)
    {
        control_accounts::touch_last_tenant(&mut conn, account.id, target_tenant_id)
            .await
            .map_err(map_db_error)?;
    }

    Ok(())
}

pub async fn ensure_dev_accounts(st: &AppState) {
    let env = std::env::var("DMSX_API_ENV")
        .unwrap_or_else(|_| "dev".to_string())
        .to_ascii_lowercase();
    if st.auth.mode != AuthMode::Jwt || env != "dev" {
        return;
    }

    let mut conn = match st.db.acquire().await {
        Ok(conn) => conn,
        Err(err) => {
            tracing::warn!("failed to acquire db connection for dev accounts: {err}");
            return;
        }
    };

    let accounts = [
        (
            "platform",
            "platform123",
            "平台管理员",
            json!(["PlatformAdmin"]),
            Some(DEV_DEFAULT_TENANT_ID),
            Some(DEV_DEFAULT_TENANT_ID),
            vec![],
        ),
        (
            "tenant",
            "tenant123",
            "单租户管理员",
            json!([]),
            Some(DEV_DEFAULT_TENANT_ID),
            Some(DEV_DEFAULT_TENANT_ID),
            vec![(DEV_DEFAULT_TENANT_ID, json!(["TenantAdmin"]))],
        ),
        (
            "hybrid",
            "hybrid123",
            "平台租户双权限管理员",
            json!(["PlatformAdmin"]),
            Some(DEV_DEFAULT_TENANT_ID),
            Some(DEV_SECOND_TENANT_ID),
            vec![
                (DEV_DEFAULT_TENANT_ID, json!(["TenantAdmin"])),
                (DEV_SECOND_TENANT_ID, json!(["Operator"])),
            ],
        ),
        (
            "multitenant",
            "multitenant123",
            "多租户运营账号",
            json!([]),
            Some(DEV_DEFAULT_TENANT_ID),
            Some(DEV_SECOND_TENANT_ID),
            vec![
                (DEV_DEFAULT_TENANT_ID, json!(["TenantAdmin"])),
                (DEV_SECOND_TENANT_ID, json!(["ReadOnly"])),
            ],
        ),
    ];

    for (
        username,
        password,
        display_name,
        platform_roles,
        default_tenant_id,
        last_tenant_id,
        tenant_entries,
    ) in accounts
    {
        match control_accounts::upsert_account(
            &mut conn,
            username,
            &hash_password(password),
            display_name,
            platform_roles,
            default_tenant_id,
            last_tenant_id,
        )
        .await
        {
            Ok(account_id) => {
                if let Err(err) = control_accounts::replace_account_tenants(
                    &mut conn,
                    account_id,
                    &tenant_entries,
                )
                .await
                {
                    tracing::warn!(username, "failed to seed dev account tenants: {err}");
                }
            }
            Err(err) => tracing::warn!(username, "failed to seed dev account: {err}"),
        }
    }
}

fn ensure_login_available(auth: &AuthConfig) -> ServiceResult<()> {
    match auth.mode {
        AuthMode::Jwt => Ok(()),
        AuthMode::Disabled => Err(dmsx_core::DmsxError::Forbidden(
            "disabled 模式不提供账号密码登录，请切换为 jwt 模式".into(),
        )),
    }
}

fn parse_roles(value: &serde_json::Value) -> ServiceResult<Vec<String>> {
    let mut roles = serde_json::from_value::<Vec<String>>(value.clone())
        .map_err(|_| dmsx_core::DmsxError::Internal("invalid stored roles".into()))?;
    roles.retain(|role| !role.trim().is_empty());
    roles.sort();
    roles.dedup();
    Ok(roles)
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}
