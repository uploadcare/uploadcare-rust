//! Provides the binding for the Uploadcare API.

use std::fmt::Debug;

use reqwest::Url;
use serde::Serialize;

mod error;
pub use error::{ErrValue, Error, Result};

#[cfg(feature = "rest")]
pub mod rest;

#[cfg(feature = "upload")]
pub mod upload;

pub(crate) const CLIENT_VERSION: &str = "0.1.0";

/// Holds per project API credentials.
/// You can find your credentials on the uploadcare dashboard.
#[derive(Debug)]
pub struct ApiCreds {
    /// API secret key
    pub secret_key: String,
    /// API public key
    pub pub_key: String,
}

pub(crate) trait IntoUrlQuery {
    fn into_query(self) -> String;
}

impl<T> IntoUrlQuery for T
where
    T: ToString,
{
    fn into_query(self) -> String {
        self.to_string()
    }
}

pub(crate) fn encode_json<T>(params: &T) -> Result<Vec<u8>, Error>
where
    T: ?Sized + Serialize,
{
    let data = serde_json::to_string(&params);
    match data {
        Err(err) => Err(Error::with_value(ErrValue::Other(err.to_string()))),
        Ok(json) => Ok(json.into_bytes()),
    }
}

pub(crate) fn encode_url<T>(base: &str, path: &str, params: Option<T>) -> Result<Url, Error>
where
    T: IntoUrlQuery,
{
    let mut u = base.to_string() + path;
    if let Some(data) = params {
        u = u + "?" + data.into_query().as_str();
    }

    let url = Url::parse(u.as_str())?;
    Ok(url)
}
