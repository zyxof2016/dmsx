pub mod auth;

mod db_rls;
mod desktop;
mod handlers;
mod limits;
mod metrics;
pub mod telemetry;
mod migrate_embedded;
mod query_types;
mod repo;

pub mod app;
pub mod desktop_helpers;
pub mod dto;
pub mod error;
pub mod helpers;
pub mod services;
pub mod state;
