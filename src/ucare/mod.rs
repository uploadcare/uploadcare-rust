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

/// Deserializes an optional integer the API may send in any of the forms a
/// media metadata field is known to arrive in: a json integer, a json float or
/// a string holding either.
///
/// The documented schema types all of these as integers, the service does not
/// always agree: the audio channel count is documented as an integer and
/// answered as `"2"`, and the ffprobe derived durations and bitrates can come
/// back fractional. Accepting every form is the only option that does not make
/// a real response fail to parse — a stricter type broke deserialization of the
/// whole file object over a single field. Fractional values are rounded to the
/// nearest integer.
///
/// `Option` alone does not make the field optional here: `deserialize_with`
/// takes over the whole field, so a `#[serde(default)]` is required next to it
/// for a missing key to come out as `None`.
pub(crate) fn de_lenient_int<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum LenientInt {
        Int(i64),
        Float(f64),
        Str(String),
    }

    const EXPECTED: &str = "an integer, a number, or a string holding one";

    match Option::<LenientInt>::deserialize(deserializer)? {
        None => Ok(None),
        Some(LenientInt::Int(value)) => Ok(Some(value)),
        Some(LenientInt::Float(value)) => float_to_int(value).map(Some).ok_or_else(|| {
            serde::de::Error::invalid_value(serde::de::Unexpected::Float(value), &EXPECTED)
        }),
        Some(LenientInt::Str(raw)) => raw
            .parse::<i64>()
            .ok()
            .or_else(|| raw.parse::<f64>().ok().and_then(float_to_int))
            .map(Some)
            .ok_or_else(|| {
                serde::de::Error::invalid_value(serde::de::Unexpected::Str(&raw), &EXPECTED)
            }),
    }
}

/// Rounds a json number to an `i64`, `None` when it does not fit into one.
fn float_to_int(value: f64) -> Option<i64> {
    let rounded = value.round();
    if rounded.is_finite() && rounded >= i64::MIN as f64 && rounded <= i64::MAX as f64 {
        Some(rounded as i64)
    } else {
        None
    }
}

/// Deserializes a map that the API documents as always present, tolerating an
/// explicit `null`.
///
/// `#[serde(default)]` alone only covers a missing key: a `"metadata": null` —
/// which webhook deliveries are documented to carry — fails the whole response
/// with "invalid type: null, expected a map". Both forms mean "nothing here".
pub(crate) fn de_null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Percent-encodes a caller supplied value that goes into the request path.
///
/// Path segments are interpolated into the url by hand, so a value holding `/`,
/// `?` or `#` would rewrite the path or the query of the request. Everything
/// outside the unreserved set of RFC 3986 is encoded here.
///
/// Dot segments cannot be encoded away — `Url::parse` normalizes `..` and its
/// percent-encoded spellings alike, turning `/files/{uuid}/metadata/../` into
/// `/files/{uuid}/` — so they are rejected instead.
#[cfg(feature = "rest")]
pub(crate) fn encode_path_segment(value: &str) -> Result<String, Error> {
    if is_dot_segment(value) {
        return Err(Error::with_value(ErrValue::BadRequest(format!(
            "invalid path segment {:?}: `.` and `..` would change the request path",
            value,
        ))));
    }

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(format!("%{:02X}", byte).as_str()),
        }
    }
    Ok(encoded)
}

/// Whether a value is a url path segment with a special meaning: `.` or `..`,
/// in any of their percent-encoded spellings.
#[cfg(feature = "rest")]
pub(crate) fn is_dot_segment(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace("%2e", ".");
    normalized == "." || normalized == ".."
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_segment_is_percent_encoded() {
        assert_eq!(encode_path_segment("remove_bg").unwrap(), "remove_bg");
        assert_eq!(
            encode_path_segment("uc_clamav_virus_scan").unwrap(),
            "uc_clamav_virus_scan",
        );
        // a value like this would otherwise rewrite the path and the query
        assert_eq!(encode_path_segment("app?x=1").unwrap(), "app%3Fx%3D1");
        assert_eq!(encode_path_segment("a/b#c").unwrap(), "a%2Fb%23c");
        assert_eq!(
            encode_path_segment("привет").unwrap(),
            "%D0%BF%D1%80%D0%B8%D0%B2%D0%B5%D1%82"
        );
    }

    #[test]
    fn path_segment_rejects_dot_segments() {
        // percent-encoding does not help here: Url::parse normalizes `%2e%2e`
        // just like `..`, so the segment has to be refused outright
        assert!(encode_path_segment(".").is_err());
        assert!(encode_path_segment("..").is_err());
        assert!(encode_path_segment("%2e%2E").is_err());

        assert!(encode_path_segment("..a").is_ok());
    }

    #[test]
    fn dot_segments_are_normalized_by_the_url_parser() {
        // what the rejection above protects from
        let url = encode_url::<String>(
            "https://api.uploadcare.com",
            "/files/1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6/metadata/../",
            None,
        )
        .unwrap();

        assert_eq!(url.path(), "/files/1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6/");
    }

    #[test]
    fn lenient_int_accepts_every_observed_form() {
        #[derive(serde::Deserialize)]
        struct Holder {
            #[serde(default, deserialize_with = "de_lenient_int")]
            value: Option<i64>,
        }

        let parse = |json: &str| serde_json::from_str::<Holder>(json).map(|h| h.value);

        assert_eq!(parse(r#"{"value": 22990}"#).unwrap(), Some(22990));
        assert_eq!(parse(r#"{"value": "22990"}"#).unwrap(), Some(22990));
        // fractional ffprobe values are rounded rather than rejected
        assert_eq!(parse(r#"{"value": 22990.5}"#).unwrap(), Some(22991));
        assert_eq!(parse(r#"{"value": "22990.4"}"#).unwrap(), Some(22990));
        assert_eq!(parse(r#"{"value": null}"#).unwrap(), None);
        assert_eq!(parse("{}").unwrap(), None);

        assert!(parse(r#"{"value": "stereo"}"#).is_err());
    }

    #[test]
    fn null_map_comes_out_empty() {
        use std::collections::HashMap;

        #[derive(serde::Deserialize)]
        struct Holder {
            #[serde(default, deserialize_with = "de_null_as_default")]
            metadata: HashMap<String, String>,
        }

        let null: Holder = serde_json::from_str(r#"{"metadata": null}"#).unwrap();
        assert!(null.metadata.is_empty());

        let missing: Holder = serde_json::from_str("{}").unwrap();
        assert!(missing.metadata.is_empty());

        let present: Holder = serde_json::from_str(r#"{"metadata": {"a": "b"}}"#).unwrap();
        assert_eq!(present.metadata.get("a"), Some(&"b".to_string()));
    }
}
