//! Holds all primitives and logic around the webhook resource.

use std::fmt::Debug;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::ucare::{encode_json, rest::Client, Result};

/// Service is used to make calls to webhook API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the webhook service
pub fn new_svc(client: &Client) -> Service {
    Service { client }
}

impl Service<'_> {
    /// Returns a list of project webhooks
    pub fn list(&self) -> Result<List> {
        self.client
            .call::<String, String, List>(Method::GET, format!("/webhooks/"), None, None)
    }

    /// Create and subscribe to webhook
    pub fn create(&self, mut params: CreateParams) -> Result<Info> {
        if params.is_active.is_none() {
            params.is_active = Some(true);
        }
        let json = encode_json(&params)?;

        self.client.call::<String, Vec<u8>, Info>(
            Method::POST,
            format!("/webhooks/"),
            None,
            Some(json),
        )
    }

    /// Update webhook attributes.
    pub fn update(&self, params: UpdateParams) -> Result<Info> {
        let json = encode_json(&params)?;

        self.client.call::<String, Vec<u8>, Info>(
            Method::PUT,
            format!("/webhooks/{}/", params.id),
            None,
            Some(json),
        )
    }

    /// Unsubscribe and delete webhook.
    pub fn delete(&self, params: DeleteParams) -> Result<()> {
        let json = encode_json(&params)?;

        let res = self.client.call::<String, Vec<u8>, String>(
            Method::DELETE,
            format!("/webhooks/unsubscribe/"),
            None,
            Some(json),
        );
        if let Err(err) = res {
            if !err.to_string().contains("EOF") {
                return Err(err);
            }
        }

        Ok(())
    }
}

/// List of webhooks returned
pub type List = Vec<Info>;

/// Webhook information
#[derive(Deserialize, Debug)]
pub struct Info {
    /// Webhook ID
    pub id: i32,
    /// Webhook creation date-time
    pub created: String,
    /// Webhook update date-time
    pub updated: String,
    /// Webhook event
    pub event: String,
    /// Where webhook data will be POSTed
    pub target_url: String,
    /// Webhook payload signing secret
    pub signing_secret: String,
    /// Webhook project ID
    pub project: i32,
    /// Whether it is active
    pub is_active: bool,
}

/// Params for creating webhook
#[derive(Debug, Serialize)]
pub struct CreateParams {
    /// An event you subscribe to.
    pub event: Event,
    /// A URL that is triggered by an event, for example, a file upload. A target URL MUST be
    /// unique for each project — event type combination.
    pub target_url: String,
    /// Payload can be signed with a secret to ensure that the request comes from the expected
    /// sender. Leave None if you don't want to change it
    pub signing_secret: Option<String>,
    /// Marks a subscription as either active or not, defaults to true, otherwise false.
    pub is_active: Option<bool>,
}

/// Events to subscribe for
#[derive(Debug, Serialize)]
pub enum Event {
    /// Fires when file is uploaded
    #[serde(rename = "file.uploaded")]
    FileUploaded,
}

/// Params for updating webhook
#[derive(Debug, Serialize)]
pub struct UpdateParams {
    /// Webhook ID
    pub id: i32,
    /// An event you subscribe to. Leave None if you don't want to change it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<Event>,
    /// A URL that is triggered by an event, for example, a file upload. A target URL MUST be
    /// unique for each project — event type combination. Leave it None if you don't want to change
    /// it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_url: Option<String>,
    /// Payload can be signed with a secret to ensure that the request comes from the expected
    /// sender
    pub signing_secret: Option<String>,
    /// Marks a subscription as either active or not, leave it None if you don't want to change it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

/// Params for deleting webhook
#[derive(Debug, Serialize)]
pub struct DeleteParams {
    /// Webhook will be found and deleted by its target_url
    pub target_url: String,
}
