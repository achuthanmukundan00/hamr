//! OAuth provider interfaces and login flows.

pub mod device_code;
pub mod oauth_page;
pub mod pkce;
pub mod registry;
pub mod types;

pub use registry::*;
