//! Error related stuff is here
//!
//! TODO: improve

use std::fmt;
use std::io;

use reqwest;
use serde::Deserialize;
use serde_json;
use url;

/// Result has Error as default value for Err value
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Represents library level error
#[derive(Deserialize)]
pub struct Error {
    detail: String,
    #[serde(skip_deserializing)]
    value: ErrValue,
}

impl Error {
    /// Error message
    pub fn detail(&self) -> String {
        self.detail.to_string()
    }

    /// Constructs error by its value
    pub fn with_value(val: ErrValue) -> Error {
        Error {
            detail: val.to_string(),
            value: val,
        }
    }

    /// Get the `ErrValue` enum for more specific error handling
    pub fn value(self) -> ErrValue {
        self.value
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Uploadcare: {}", self.value)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {}", self.value)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error {
            detail: err.to_string(),
            value: ErrValue::InputOutput(err),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error {
            detail: err.to_string(),
            value: ErrValue::Reqwest(err),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error {
            detail: err.to_string(),
            value: ErrValue::SerdeJson(err),
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error {
            detail: err.to_string(),
            value: ErrValue::ParseUrl(err),
        }
    }
}

/// Represents possible errors returned by the library
pub enum ErrValue {
    /// Endpoint parameters error
    BadRequest(String),
    /// Authorization errors
    Unauthorized(String),
    /// Forbidden error
    Forbidden(String),
    /// Not found error
    NotFound(String),
    /// Invalid version header `Accept` for the endpoint
    NotAcceptable(String),
    /// Payload too large
    PayloadTooLarge(String),
    /// Request was throttled
    TooManyRequests(i32),

    /// Errors returned from reqwest underlying lib
    Reqwest(reqwest::Error),
    /// Errors returned from io
    InputOutput(io::Error),
    /// JSON serialization/deserialization errors
    SerdeJson(serde_json::Error),
    /// Url parsing errors
    ParseUrl(url::ParseError),

    /// Other custom errors
    Other(String),
}

impl fmt::Display for ErrValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prefix = "Uploadcare";

        match *self {
            ErrValue::BadRequest(ref msg) => write!(f, "{}: {}", prefix, msg),
            ErrValue::Unauthorized(ref msg) => write!(f, "{}: {}", prefix, msg),
            ErrValue::Forbidden(ref msg) => write!(f, "{}: {}", prefix, msg),
            ErrValue::NotFound(ref msg) => write!(f, "{}: {}", prefix, msg),
            ErrValue::NotAcceptable(ref msg) => write!(f, "{}: {}", prefix, msg),
            ErrValue::PayloadTooLarge(ref msg) => write!(f, "{}: {}", prefix, msg),
            ErrValue::TooManyRequests(ref retry_after) => write!(
                f,
                "{}: too many requests, retry after {}",
                prefix, retry_after
            ),

            ErrValue::Reqwest(ref err) => write!(f, "{}: {}", prefix, err),
            ErrValue::InputOutput(ref err) => write!(f, "{}: {}", prefix, err),
            ErrValue::SerdeJson(ref err) => write!(f, "{}: {}", prefix, err),
            ErrValue::ParseUrl(ref err) => write!(f, "{}: {}", prefix, err),

            ErrValue::Other(ref msg) => write!(f, "{}: {}", prefix, msg),
        }
    }
}

impl Default for ErrValue {
    fn default() -> Self {
        ErrValue::Other("ErrValue".to_string())
    }
}
