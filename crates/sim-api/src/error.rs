//! The one error type of the wire layer: `{ code, message }`.
use serde::{Deserialize, Serialize};

/// Machine-readable error codes. The UI switches on `code`; `message` is for logs and the
/// recoverable-error screen.
pub mod code {
    pub const INIT_FAILED: &str = "init_failed";
    pub const BAD_METADATA: &str = "bad_metadata";
    pub const UNSUPPORTED_PARAMS: &str = "unsupported_params";
    pub const BAD_REQUEST: &str = "bad_request";
    pub const GAME_OVER: &str = "game_over";
    pub const NO_SUCH_PENDING: &str = "no_such_pending";
    pub const VIOLATION: &str = "violation";
    pub const SCHEMA_MISMATCH: &str = "schema_mismatch";
    pub const PARAMS_MISMATCH: &str = "params_mismatch";
    pub const TURN_STATE: &str = "turn_state";
    pub const BAD_STATE: &str = "bad_state";
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        ApiError {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

pub type ApiResult<T> = Result<T, ApiError>;
