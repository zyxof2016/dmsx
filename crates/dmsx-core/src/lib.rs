//! 共享领域类型与错误。表结构见 `migrations/` 与 `docs/DOMAIN_MODEL.md`。

pub mod domain;
pub mod error;

pub use domain::*;
pub use error::DmsxError;
