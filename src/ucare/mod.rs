//! Provides the binding for the Uploadcare API.

use std::fmt::Debug;

use reqwest::Url;
use serde::{Deserialize, Serialize};

mod error;
pub use error::{ErrValue, Error, Result};

#[cfg(feature = "rest")]
pub mod rest;

#[cfg(feature = "upload")]
pub mod upload;

/// Version reported in the `X-UC-User-Agent` header. Taken from the crate
/// manifest so it never drifts away from the published version.
#[cfg(feature = "rest")]
pub(crate) const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[cfg(feature = "rest")]
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

/// Percent-encodes a single query parameter value.
///
/// `into_query` implementations concatenate `key=value` pairs by hand, so any
/// user supplied value (an ISO 8601 cursor with `+03:00`, a request id) must be
/// encoded here or characters like `+`, `&` and `#` change the request.
#[cfg(feature = "rest")]
pub(crate) fn encode_query_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Deserializes an optional integer that the API may send either as a json
/// number or as a string holding one.
///
/// The media metadata is where this happens: the documented schema types the
/// audio channel count as an integer, the service answers with `"2"`. Accepting
/// both is the only option that does not make a real response fail to parse —
/// the alternative was a `String` field, which then broke for every response
/// that does follow the schema.
///
/// `Option` alone does not make the field optional here: `deserialize_with`
/// takes over the whole field, so a `#[serde(default)]` is required next to it
/// for a missing key to come out as `None`.
pub(crate) fn de_int_or_string<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum IntOrString {
        Int(i64),
        Str(String),
    }

    match Option::<IntOrString>::deserialize(deserializer)? {
        None => Ok(None),
        Some(IntOrString::Int(value)) => Ok(Some(value)),
        Some(IntOrString::Str(raw)) => raw.parse::<i64>().map(Some).map_err(|_| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(&raw),
                &"an integer, or a string holding one",
            )
        }),
    }
}

pub(crate) fn encode_url<T>(base: &str, path: &str, params: Option<T>) -> Result<Url, Error>
where
    T: IntoUrlQuery,
{
    let mut u = base.to_string() + path;
    if let Some(data) = params {
        let query = data.into_query();
        if !query.is_empty() {
            u = u + "?" + query.as_str();
        }
    }

    let url = Url::parse(u.as_str())?;
    Ok(url)
}
