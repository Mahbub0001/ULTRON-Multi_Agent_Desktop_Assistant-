//! gRPC Service Implementations

pub mod input_service;
pub mod macro_service;
pub mod task_service;
pub mod auth_service;
pub mod system_service;

use tonic::{Request, Response, Status};
use crate::proto::hcs::agent::v1::*;

// Re-export services
pub use input_service::InputServiceImpl;
pub use macro_service::MacroServiceImpl;
pub use task_service::TaskServiceImpl;
pub use auth_service::AuthServiceImpl;
pub use system_service::SystemServiceImpl;