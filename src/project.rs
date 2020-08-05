//! Holds all primitives and logic around the project resource.

use std::fmt::Debug;

use reqwest::Method;
use serde::Deserialize;

use crate::ucare::{rest::Client, Result};

/// Service is used to make calls to webhook API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the webhook service
pub fn new_svc(client: &Client) -> Service {
    Service { client }
}

impl Service<'_> {
    /// Getting info about account project.
    pub fn info(&self) -> Result<Info> {
        self.client
            .call::<String, String, Info>(Method::GET, format!("/project/"), None, None)
    }
}

/// Account project information
#[derive(Debug, Deserialize)]
pub struct Info {
    /// Project login name.
    pub name: String,
    /// Project public key.
    pub pub_key: String,
    /// Project collaborators.
    pub collaborators: Option<Vec<Collaborator>>,
}

/// Collaborator information
#[derive(Debug, Deserialize)]
pub struct Collaborator {
    /// Collaborator email.
    pub email: String,
    /// Collaborator name.
    pub name: String,
}
