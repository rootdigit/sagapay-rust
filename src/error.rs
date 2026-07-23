use thiserror::Error;

/// SagaPay error types
#[derive(Error, Debug)]
pub enum Error {
    /// Network error
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// API error
    ///
    /// `message` is the human readable text from the response body's `error`
    /// field. `error_code` is reserved for a machine readable code, which the
    /// API does not currently return.
    #[error("API error: {status_code} - {message}")]
    ApiError {
        status_code: u16,
        message: String,
        error_code: Option<String>,
    },

    /// Missing required parameter
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    /// Invalid parameter value
    #[error("Invalid parameter value: {0}")]
    InvalidParameter(String),

    /// Invalid or missing webhook signature
    #[error("Invalid webhook signature")]
    InvalidSignature,

    /// Other errors
    #[error("{0}")]
    Other(String),
}
