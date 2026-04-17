pub mod artifacts;
pub mod bootstrap;
pub mod commands;
pub mod compliance;
pub mod devices;
pub mod hierarchy;
pub mod policies;
pub mod audit;
pub mod system_settings;
pub mod shadow;
pub mod stats;

use dmsx_core::DmsxError;

pub type ServiceResult<T> = Result<T, DmsxError>;
