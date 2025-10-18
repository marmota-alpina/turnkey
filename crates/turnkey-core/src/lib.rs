pub mod constants;
pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

/// Version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
