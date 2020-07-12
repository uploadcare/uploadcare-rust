//! Provides a client for Uploadcare Upload API

use std::fmt::{self, Debug};

use log::debug;
use reqwest::{
    blocking::{multipart::Form, Client as http_client, ClientBuilder},
    header, Method, StatusCode, Url,
};
use serde::Deserialize;

use super::{encode_url, ApiCreds, ErrValue, Error, IntoUrlQuery, Result};

pub(crate) mod auth;
pub(crate) use auth::Fields;

const API_URL: &str = "https://upload.uploadcare.com";

/// Configuration for the client.
#[derive(Debug)]
pub struct Config {
    /// Should be true if you want to use signed uploads
    pub sign_based_upload: bool,
}

pub(crate) enum Payload {
    Form(Form),
    Raw(Vec<u8>),
}

/// Client is responsible for preparing requests and making http calls.
pub struct Client {
    pub(crate) auth_fields: Box<dyn Fn() -> auth::Fields>,

    client: http_client,
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Client {{}}")
    }
}

impl Client {
    /// Initializes new client instance
    pub fn new(config: Config, creds: ApiCreds) -> Result<Self, String> {
        if creds.secret_key.is_empty() || creds.pub_key.is_empty() {
            return Err("Uploadcare: invalid api credentials provided".to_string());
        }

        let http_client = ClientBuilder::new().build().unwrap();

        let client = Client {
            auth_fields: if config.sign_based_upload {
                Box::new(auth::sign_based(creds))
            } else {
                Box::new(auth::simple(creds))
            },

            client: http_client,
        };

        Ok(client)
    }

    /// makes actual http request
    pub(crate) fn call<Q, R>(
        &self,
        method: Method,
        path: String,
        query: Option<Q>,
        data: Option<Payload>,
    ) -> Result<R, Error>
    where
        Q: IntoUrlQuery,
        for<'de> R: Deserialize<'de> + Default,
    {
        let url = encode_url::<Q>(API_URL, path.as_str(), query)?;
        self.call_url::<R>(method, url, data)
    }

    pub(crate) fn call_url<R>(
        &self,
        method: Method,
        url: Url,
        data: Option<Payload>,
    ) -> Result<R, Error>
    where
        for<'de> R: Deserialize<'de> + Default,
    {
        let mut req_builder = self.client.request(method, url);
        if let Some(body_data) = data {
            match body_data {
                Payload::Form(form) => {
                    req_builder = req_builder.multipart(form);
                }
                Payload::Raw(data) => {
                    req_builder = req_builder
                        .body(data)
                        .header(header::CONTENT_TYPE, "application/octet-stream");
                }
            }
        }
        let req = req_builder.build()?;

        debug!("created new request: {:?}", req);
        let res = self.client.execute(req)?;
        debug!("received response: {:?}", res);

        match res.status() {
            StatusCode::BAD_REQUEST => Err(Error::with_value(ErrValue::BadRequest(
                res.text_with_charset("utf-8")?,
            ))),
            StatusCode::FORBIDDEN => Err(Error::with_value(ErrValue::Forbidden(
                res.text_with_charset("utf-8")?,
            ))),
            StatusCode::NOT_FOUND => Err(Error::with_value(ErrValue::NotFound(
                res.text_with_charset("utf-8")?,
            ))),
            StatusCode::PAYLOAD_TOO_LARGE => Err(Error::with_value(ErrValue::PayloadTooLarge(
                res.text_with_charset("utf-8")?,
            ))),
            // picking 30 seconds because retry-after is not returned from the API
            StatusCode::TOO_MANY_REQUESTS => Err(Error::with_value(ErrValue::TooManyRequests(30))),
            StatusCode::OK | _ => match res.json() {
                Ok(data) => Ok(data),
                Err(err) => {
                    if err.to_string().contains("EOF") {
                        Ok(R::default())
                    } else {
                        Err(Error::from(err))
                    }
                }
            },
        }
    }
}
