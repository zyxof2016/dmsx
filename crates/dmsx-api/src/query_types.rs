#[derive(Debug, sqlx::FromRow)]
pub struct CountBucketRow {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StatsRow {
    pub device_total: i64,
    pub device_online: i64,
    pub policy_count: i64,
    pub command_pending: i64,
    pub finding_open: i64,
}
