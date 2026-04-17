//! Simple per-tenant rate limiting for device-gw (internal beta).
//!
//! This is an in-memory limiter. For multi-replica fairness, rely on LB + per-pod limits.

use std::num::NonZeroU32;
use std::sync::Arc;

use governor::{Quota, RateLimiter};
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;
use tonic::Status;
use uuid::Uuid;

pub type TenantLimiter = RateLimiter<Uuid, DefaultKeyedStateStore<Uuid>, DefaultClock, NoOpMiddleware>;

fn truthy_env(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

fn per_second_from_env() -> u32 {
    std::env::var("DMSX_GW_RATE_LIMIT_PER_SECOND")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(100)
        .max(1)
}

fn burst_from_env() -> u32 {
    std::env::var("DMSX_GW_RATE_LIMIT_BURST")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(200)
        .max(1)
}

pub fn from_env() -> Option<Arc<TenantLimiter>> {
    if !truthy_env("DMSX_GW_RATE_LIMIT_ENABLED") {
        return None;
    }
    let per_second = per_second_from_env();
    let burst = burst_from_env();
    let quota = Quota::per_second(NonZeroU32::new(per_second).unwrap())
        .allow_burst(NonZeroU32::new(burst).unwrap());
    Some(Arc::new(RateLimiter::keyed(quota)))
}

pub fn check(limiter: &Option<Arc<TenantLimiter>>, tenant_id: Uuid) -> Result<(), Status> {
    let Some(l) = limiter else {
        return Ok(());
    };
    if l.check_key(&tenant_id).is_ok() {
        return Ok(());
    }
    Err(Status::resource_exhausted("rate limit exceeded"))
}

