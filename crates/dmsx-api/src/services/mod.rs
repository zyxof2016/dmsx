pub mod artifacts;
pub mod bootstrap;
pub mod commands;
pub mod compliance;
pub mod devices;
pub mod policies;
pub mod shadow;
pub mod stats;

use dmsx_core::DmsxError;

pub type ServiceResult<T> = Result<T, DmsxError>;
