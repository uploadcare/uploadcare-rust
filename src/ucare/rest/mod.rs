//! Provides a client for Uploadcare REST API

use std::fmt::{self, Debug};

use chrono::Utc;
use log::debug;
use reqwest::{
    blocking::{Body, Client as http_client, ClientBuilder, Request},
    header, Method, StatusCode, Url,
};
use serde::Deserialize;

use super::{encode_url, ApiCreds, ErrValue, Error, IntoUrlQuery, CLIENT_VERSION};

mod auth;

const USER_AGENT_PREFIX: &str = "UploadcareRust";
const API_URL: &str = "https://api.uploadcare.com";

/// Available API versions for client to specify when making requests.
#[derive(Debug)]
pub enum ApiVersion {
    /// API version v0.5
    V05,
    /// API version v0.6
    V06,
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ApiVersion::V05 => write!(f, "v0.5"),
            ApiVersion::V06 => write!(f, "v0.6"),
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
                format!("application/vnd.uploadcare-{}+json", &config.api_version).as_str(),
            )
            .unwrap(),
        );

        headers.insert(
            "X-UC-User-Agent",
            header::HeaderValue::from_str(
                format!(
                    "{}/{}/{}",
                    USER_AGENT_PREFIX, CLIENT_VERSION, &creds.pub_key
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
                Utc::now()
                    .format(auth::DATE_HEADER_FORMAT)
                    .to_string()
                    .replace("UTC", "GMT"),
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

        match res.status() {
            StatusCode::BAD_REQUEST => Err(Error::with_value(ErrValue::BadRequest(
                res.json::<Error>()?.detail(),
            ))),
            StatusCode::UNAUTHORIZED => Err(Error::with_value(ErrValue::Unauthorized(
                res.json::<Error>()?.detail(),
            ))),
            StatusCode::NOT_ACCEPTABLE => Err(Error::with_value(ErrValue::NotAcceptable(
                res.json::<Error>()?.detail(),
            ))),
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = res.headers()[header::RETRY_AFTER]
                    .to_str()
                    .unwrap()
                    .parse::<i32>()
                    .unwrap();
                Err(Error::with_value(ErrValue::TooManyRequests(retry_after)))
            }
            StatusCode::OK | _ => {
                let resp_data: R = res.json()?;
                Ok(resp_data)
            }
        }
    }
}
