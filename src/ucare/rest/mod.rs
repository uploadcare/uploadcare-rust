//! Provides a client for Uploadcare REST API

use std::fmt::{self, Debug};

use chrono::Utc;
use log::{debug, warn};
use reqwest::{
    blocking::{Body, Client as http_client, ClientBuilder, Request, Response},
    header, Method, StatusCode, Url,
};
use serde::Deserialize;

use super::{encode_url, ApiCreds, ErrValue, Error, IntoUrlQuery, CLIENT_VERSION};

mod auth;

const USER_AGENT_PREFIX: &str = "UploadcareRust";
const API_URL: &str = "https://api.uploadcare.com";
/// Error response bodies longer than that are cut before being put into an `Error`.
const MAX_ERROR_BODY_LEN: usize = 512;
/// Reported by `ErrValue::TooManyRequests` when the `Retry-After` header of a
/// `429` is missing or not a plain number of seconds. Same value as the Upload
/// API client uses.
const DEFAULT_RETRY_AFTER_SECS: i32 = 30;

/// Available API versions for client to specify when making requests.
///
/// Non exhaustive on purpose: API versions come and go, and matching on this
/// enum downstream must not break when the next one is added.
#[derive(Debug)]
#[non_exhaustive]
pub enum ApiVersion {
    /// API version v0.7
    V07,
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ApiVersion::V07 => write!(f, "v0.7"),
        }
    }
}

/// Configuration for the client.
#[derive(Debug)]
pub struct Config {
    /// Should be true if you want to use signature based authentication for the
    /// REST API calls.
    pub sign_based_auth: bool,
    /// REST API version to be used.
    pub api_version: ApiVersion,
}

/// Client is responsible for preparing requests and making http calls.
pub struct Client {
    set_auth_header: Box<dyn Fn(&mut Request)>,

    client: http_client,
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Client {{}}")?;
        Ok(())
    }
}

impl Client {
    /// Initializes new client instance
    pub fn new(config: Config, creds: ApiCreds) -> Result<Self, String> {
        if creds.secret_key.is_empty() || creds.pub_key.is_empty() {
            return Err("Uploadcare: invalid api credentials provided".to_string());
        }

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_str(
                format!("application/vnd.uploadcare-{}+json", config.api_version).as_str(),
            )
            .unwrap(),
        );

        headers.insert(
            "X-UC-User-Agent",
            header::HeaderValue::from_str(
                format!(
                    "{}/{}/{}",
                    USER_AGENT_PREFIX, CLIENT_VERSION, creds.pub_key
                )
                .as_str(),
            )
            .unwrap(),
        );

        let http_client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        let client = Client {
            set_auth_header: if config.sign_based_auth {
                Box::new(auth::sign_based(creds))
            } else {
                Box::new(auth::simple(creds))
            },

            client: http_client,
        };

        Ok(client)
    }

    /// makes actual http request
    pub(crate) fn call<Q, D, R>(
        &self,
        method: Method,
        path: String,
        query: Option<Q>,
        data: Option<D>,
    ) -> Result<R, Error>
    where
        D: Sized + Into<Body>,
        Q: IntoUrlQuery,
        for<'de> R: Deserialize<'de>,
    {
        let url = encode_url::<Q>(API_URL, path.as_str(), query)?;
        self.call_url::<D, R>(method, url, data)
    }

    pub(crate) fn call_url<D, R>(
        &self,
        method: Method,
        url: Url,
        data: Option<D>,
    ) -> Result<R, Error>
    where
        D: Sized + Into<Body>,
        for<'de> R: Deserialize<'de>,
    {
        let mut req_builder = self
            .client
            .request(method, url)
            .header(
                header::DATE,
                Utc::now().format(auth::DATE_HEADER_FORMAT).to_string(),
            )
            .header(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            );
        if let Some(body_data) = data {
            req_builder = req_builder.body(body_data);
        }
        let mut req = req_builder.build()?;

        (*self.set_auth_header)(&mut req);

        debug!("created new request: {:?}", req);
        let res = self.client.execute(req)?;
        debug!("received response: {:?}", res);

        log_warnings(&res);

        match res.status() {
            StatusCode::BAD_REQUEST => Err(Error::with_value(ErrValue::BadRequest(error_detail(
                res,
                "bad request",
            )))),
            StatusCode::UNAUTHORIZED => Err(Error::with_value(ErrValue::Unauthorized(
                error_detail(res, "unauthorized"),
            ))),
            StatusCode::FORBIDDEN => Err(Error::with_value(ErrValue::Forbidden(error_detail(
                res,
                "forbidden",
            )))),
            StatusCode::NOT_FOUND => Err(Error::with_value(ErrValue::NotFound(error_detail(
                res,
                "not found",
            )))),
            StatusCode::METHOD_NOT_ALLOWED => Err(Error::with_value(ErrValue::MethodNotAllowed(
                error_detail(res, "method not allowed"),
            ))),
            StatusCode::NOT_ACCEPTABLE => Err(Error::with_value(ErrValue::NotAcceptable(
                error_detail(res, "not acceptable"),
            ))),
            StatusCode::CONFLICT => Err(Error::with_value(ErrValue::Conflict(error_detail(
                res, "conflict",
            )))),
            StatusCode::PAYLOAD_TOO_LARGE => Err(Error::with_value(ErrValue::PayloadTooLarge(
                error_detail(res, "payload too large"),
            ))),
            StatusCode::TOO_MANY_REQUESTS => {
                // the header is expected here, but a missing or malformed one
                // (an HTTP-date, or a proxy that stripped it) is not worth a
                // panic — and must not come out as 0 either, which would tell a
                // caller sleeping for this long to retry immediately
                let retry_after = res
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<i32>().ok())
                    .filter(|secs| *secs > 0)
                    .unwrap_or(DEFAULT_RETRY_AFTER_SECS);
                Err(Error::with_value(ErrValue::TooManyRequests(retry_after)))
            }
            status if status.is_server_error() => Err(Error::with_value(ErrValue::ServerError(
                status.as_u16(),
                error_detail(res, status.canonical_reason().unwrap_or("server error")),
            ))),
            status if status.is_success() => {
                // 204 responses (delete endpoints) and other empty bodies are
                // deserialized from JSON `null`, so `serde_json::Value` and
                // `Option<T>` targets succeed instead of hitting a serde EOF
                let body = res.text()?;
                let body = body.trim();
                if body.is_empty() {
                    Ok(serde_json::from_str("null")?)
                } else {
                    Ok(serde_json::from_str(body)?)
                }
            }
            // redirects and anything else we do not know about: reporting the
            // status instead of feeding the body to the deserializer
            status => Err(Error::with_value(ErrValue::Other(format!(
                "unexpected response status {}: {}",
                status,
                error_detail(res, "empty response body"),
            )))),
        }
    }
}

/// Logs every `Warning` header returned by the API.
///
/// APIv0.7 uses it to report non fatal problems with an otherwise successful
/// request, e.g. metadata keys dropped as invalid on `local_copy`.
fn log_warnings(res: &Response) {
    for value in res.headers().get_all(header::WARNING).iter() {
        match value.to_str() {
            Ok(warning) => warn!("uploadcare api warning: {}", warning),
            Err(_) => warn!("uploadcare api warning (non utf-8): {:?}", value.as_bytes()),
        }
    }
}

/// Builds a readable message out of an error response body.
///
/// Most of the API errors come as `{"detail": "..."}`, but not all of them do:
/// 404/405 may carry an empty body and a 5xx may be an html page produced by an
/// intermediate proxy. Deserializing those is what used to surface a serde
/// error instead of the actual http one, so whatever does not look like the
/// known json payload is passed through as raw text.
fn error_detail(res: Response, fallback: &str) -> String {
    match res.text() {
        Ok(body) => detail_from_body(body.as_str(), fallback),
        Err(_) => fallback.to_string(),
    }
}

/// The body parsing part of [`error_detail`], split out to be testable.
fn detail_from_body(body: &str, fallback: &str) -> String {
    if let Ok(err) = serde_json::from_str::<Error>(body) {
        return err.detail();
    }

    let body = body.trim();
    if body.is_empty() {
        return fallback.to_string();
    }
    if body.chars().count() > MAX_ERROR_BODY_LEN {
        let mut cut: String = body.chars().take(MAX_ERROR_BODY_LEN).collect();
        cut.push_str("... (truncated)");
        return cut;
    }
    body.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_accept_header() {
        assert_eq!(ApiVersion::V07.to_string(), "v0.7");
    }

    #[test]
    fn test_detail_from_json_body() {
        assert_eq!(
            detail_from_body(r#"{"detail": "Method not allowed."}"#, "fallback"),
            "Method not allowed.",
        );
    }

    #[test]
    fn test_detail_from_empty_body() {
        // 404/405 responses may carry no body at all
        assert_eq!(detail_from_body("", "not found"), "not found");
        assert_eq!(detail_from_body("  \n ", "not found"), "not found");
    }

    #[test]
    fn test_detail_from_non_json_body() {
        // proxy generated 5xx pages are not json, they must not end up as a
        // serde error
        assert_eq!(
            detail_from_body("<html><body>502 Bad Gateway</body></html>", "server error"),
            "<html><body>502 Bad Gateway</body></html>",
        );
    }

    #[test]
    fn test_detail_from_json_without_detail_field() {
        assert_eq!(
            detail_from_body(r#"{"error": "oops"}"#, "bad request"),
            r#"{"error": "oops"}"#,
        );
    }

    #[test]
    fn test_detail_is_truncated() {
        let detail = detail_from_body("x".repeat(MAX_ERROR_BODY_LEN + 100).as_str(), "fallback");

        assert_eq!(detail.len(), MAX_ERROR_BODY_LEN + "... (truncated)".len());
        assert!(detail.ends_with("... (truncated)"));
    }
}
