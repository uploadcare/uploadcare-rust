//! Holds all primitives and logic around the webhook resource.
//!
//! Two independent notions of version are involved here, do not mix them up:
//!
//! - the **request** version, the `Accept` header configured on the client. It decides
//!   which subscription versions the API accepts for this request.
//! - the **subscription** version, [`Info::version`]. It decides which events can be
//!   subscribed to and in which format deliveries arrive at the target url.
//!
//! The subscription version is fixed when the subscription is created and can never be
//! changed afterwards; the only way to "change" it is deleting the subscription and
//! creating it anew. Reading the list of subscriptions can therefore return a mix of
//! versions, which is why [`Info::version`] is a plain string rather than an enum.
//!
//! Since this crate speaks APIv0.7 only, [`Version::V07`] is the sole value it can
//! create subscriptions with, and [`CreateParams`] always sends it explicitly.

use std::fmt::{self, Debug, Display};

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::ucare::{encode_json, rest::Client, Result};

/// Service is used to make calls to webhook API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the webhook service
pub fn new_svc(client: &Client) -> Service<'_> {
    Service { client }
}

impl Service<'_> {
    /// Returns a list of project webhooks
    ///
    /// May well contain subscriptions of different versions: the ones created before
    /// the project moved to APIv0.7 keep theirs.
    ///
    /// Note that webhook availability is checked on every request to this resource,
    /// including this one, and the check takes the target url into account. So a
    /// `ErrValue::Forbidden` here or on one particular [`Service::create`] call says
    /// nothing about the other addresses — do not cache it as a per project flag.
    pub fn list(&self) -> Result<List> {
        self.client
            .call::<String, String, List>(Method::GET, "/webhooks/".to_string(), None, None)
    }

    /// Returns a single webhook by its id
    ///
    /// Note: `GET /webhooks/{id}/` is not part of the documented contract (the
    /// docs list exactly four webhook operations). It works, but being
    /// undocumented it comes with no compatibility promise — when that matters,
    /// use [`Service::list`] and filter.
    pub fn get(&self, id: i32) -> Result<Info> {
        self.client.call::<String, String, Info>(
            Method::GET,
            format!("/webhooks/{}/", id),
            None,
            None,
        )
    }

    /// Create and subscribe to webhook
    ///
    /// The `event` + `target_url` + project triple is unique. Subscribing twice
    /// answers `400` with `This project is already subscribed on this event`, which
    /// usually means "already subscribed" rather than a real failure. The same
    /// `target_url` may serve several different events without conflict.
    ///
    /// `target_url` is validated at request time: `http` and `https` only, absolute,
    /// with a host that has to resolve to a non private address. Loopback and
    /// private range addresses are rejected, so a local endpoint cannot be used for
    /// debugging — use a publicly reachable address or a tunnel.
    pub fn create(&self, mut params: CreateParams) -> Result<Info> {
        if params.version.is_none() {
            params.version = Some(Version::V07);
        }
        let json = encode_json(&params)?;

        self.client.call::<String, Vec<u8>, Info>(
            Method::POST,
            "/webhooks/".to_string(),
            None,
            Some(json),
        )
    }

    /// Update webhook attributes.
    ///
    /// The update is partial, only the fields that are set are sent. The subscription
    /// version is deliberately absent from [`UpdateParams`]: changing it is rejected
    /// by the API with `WebHook version updates are not allowed.`
    ///
    /// The typical use is re-enabling a subscription the API disabled on its own:
    /// after repeated delivery failures it sets `is_active` to false and notifies the
    /// account billing address, and `is_active: Some(true)` here brings it back.
    /// Events missed while it was off are not replayed.
    pub fn update(&self, params: UpdateParams) -> Result<Info> {
        let json = encode_json(&params)?;

        self.client.call::<String, Vec<u8>, Info>(
            Method::PUT,
            format!("/webhooks/{}/", params.id),
            None,
            Some(json),
        )
    }

    /// Unsubscribe from a target url.
    ///
    /// Removes **every** subscription pointing at this `target_url`, across all of the
    /// events at once — it is not a way to drop a single event. To remove one
    /// subscription of several sharing an address, there is no dedicated endpoint;
    /// this call takes all of them.
    ///
    /// Unsubscribing from an address that has no subscriptions is not an error.
    pub fn delete(&self, params: DeleteParams) -> Result<()> {
        let json = encode_json(&params)?;

        // the body has to travel with a DELETE here, which is unusual enough that
        // some http clients drop it; reqwest attaches it regardless of the method,
        // and a body-less request would be answered with `\`target_url\` is missing`
        self.client.call::<String, Vec<u8>, ()>(
            Method::DELETE,
            "/webhooks/unsubscribe/".to_string(),
            None,
            Some(json),
        )
    }
}

/// List of webhooks returned
pub type List = Vec<Info>;

/// Webhook information
#[derive(Deserialize)]
pub struct Info {
    /// Webhook ID. An integer, not a UUID, unlike file and group identifiers.
    pub id: i32,
    /// Webhook creation date-time
    pub created: String,
    /// Webhook update date-time
    pub updated: String,
    /// Webhook event
    ///
    /// A plain string on purpose: reading existing subscriptions can turn up an event
    /// this version of the crate does not know about yet, and that must not break
    /// deserialization. See [`Event`] for the values that can be subscribed to.
    pub event: String,
    /// Where webhook data will be POSTed
    pub target_url: String,
    /// Webhook payload signing secret, if one was set
    ///
    /// This is a value the client chooses, and the API returns it in responses. Treat
    /// it as a secret: the [`Debug`] implementation of this struct masks it, but
    /// anything that reads the field directly has to take care of that itself.
    pub signing_secret: Option<String>,
    /// Webhook project ID. Set by the API from the request credentials.
    pub project: i32,
    /// Whether it is active
    ///
    /// The API may clear this on its own after repeated delivery failures — that is
    /// the usual reason for deliveries to stop arriving. The subscription itself is
    /// kept, so [`Service::update`] with `is_active: Some(true)` re-enables it.
    pub is_active: bool,
    /// Subscription version, `0.7` for everything this crate creates
    ///
    /// Fixed at creation time and never changes, so older subscriptions keep
    /// reporting the version they were made with. A plain string for that reason.
    pub version: String,
}

impl Debug for Info {
    /// Masks `signing_secret` so it does not end up in logs or debug output.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Info")
            .field("id", &self.id)
            .field("created", &self.created)
            .field("updated", &self.updated)
            .field("event", &self.event)
            .field("target_url", &self.target_url)
            .field(
                "signing_secret",
                &self.signing_secret.as_ref().map(|_| "<masked>"),
            )
            .field("project", &self.project)
            .field("is_active", &self.is_active)
            .field("version", &self.version)
            .finish()
    }
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
    /// sender. Optional, not sent when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_secret: Option<String>,
    /// Marks a subscription as either active or not. Not sent when `None`, the
    /// API default is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    /// Subscription version. Defaults to [`Version::V07`] when left None.
    ///
    /// Always sent explicitly so that the created subscription does not silently
    /// depend on which `Accept` version the client happens to send.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
}

/// Version of a webhook subscription
///
/// Only `0.7` can be created through APIv0.7: passing `0.6` is answered with `400`
/// `Invalid version`. Older subscriptions keep their own version — read it from
/// [`Info::version`], which is a plain string for that reason.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[non_exhaustive]
pub enum Version {
    /// "0.7"
    #[serde(rename = "0.7")]
    V07,
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Version::V07 => write!(f, "0.7"),
        }
    }
}

/// Events to subscribe for
///
/// All of these require a `0.7` subscription. A `0.6` one only supports
/// [`Event::FileUploaded`], but since this crate cannot create anything but `0.7`
/// subscriptions, that combination is not reachable through it.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[non_exhaustive]
pub enum Event {
    /// Fires when file is uploaded
    #[serde(rename = "file.uploaded")]
    FileUploaded,
    /// Fires when a threat is found in a file by the virus scan. Added in APIv0.7.
    #[serde(rename = "file.infected")]
    FileInfected,
    /// Fires when a file is moved to permanent storage. Added in APIv0.7.
    #[serde(rename = "file.stored")]
    FileStored,
    /// Fires when a file is marked as removed. Added in APIv0.7.
    #[serde(rename = "file.deleted")]
    FileDeleted,
    /// Fires when data accompanying a file changes, but not its content: metadata,
    /// tags or application results. Added in APIv0.7.
    ///
    /// Carries the list of changed attribute groups together with their previous
    /// values. Two things to keep in mind: no event is published when nothing
    /// actually changed, so this is not a confirmation that a write happened; and
    /// application results change as a consequence of background processing that may
    /// well not have been requested by your code.
    #[serde(rename = "file.info_updated")]
    FileInfoUpdated,
}

/// Params for updating webhook
#[derive(Debug, Serialize)]
pub struct UpdateParams {
    /// Webhook ID. Identifies the subscription in the request path; not part
    /// of the documented request body, hence never serialized into it.
    #[serde(skip_serializing)]
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
    /// sender. Leave it `None` to keep the current secret: the field is only
    /// sent when set, so an unset secret can never wipe an existing one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_secret: Option<String>,
    /// Marks a subscription as either active or not, leave it None if you don't want to change it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

/// Params for deleting webhook
#[derive(Debug, Serialize)]
pub struct DeleteParams {
    /// Every subscription pointing at this target_url will be removed, for all of the
    /// events at once
    pub target_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_json() -> &'static str {
        r#"{
            "id": 1387,
            "created": "2026-08-04T10:00:00.123456Z",
            "updated": "2026-08-04T10:00:00.123456Z",
            "event": "file.info_updated",
            "target_url": "https://example.com/uploadcare/hook",
            "project": 13,
            "is_active": true,
            "signing_secret": "s3cr3t",
            "version": "0.7"
        }"#
    }

    #[test]
    fn info_deserializes() {
        let info: Info = serde_json::from_str(info_json()).unwrap();

        assert_eq!(info.id, 1387);
        assert_eq!(info.version, "0.7");
        assert_eq!(info.event, "file.info_updated");
        assert_eq!(info.signing_secret, Some("s3cr3t".to_string()));
    }

    #[test]
    fn info_accepts_null_signing_secret() {
        let json =
            info_json().replace(r#""signing_secret": "s3cr3t""#, r#""signing_secret": null"#);
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(info.signing_secret, None);
    }

    #[test]
    fn info_accepts_older_subscription_versions() {
        // the list can hold a mix: subscriptions created before the move to v0.7
        // keep their own version and their own delivery format
        let json = info_json()
            .replace(r#""version": "0.7""#, r#""version": "0.6""#)
            .replace(
                r#""event": "file.info_updated""#,
                r#""event": "file.uploaded""#,
            );
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(info.version, "0.6");
    }

    #[test]
    fn info_accepts_unknown_event() {
        // event is a plain string so a value this version does not know about does
        // not break reading the list
        let json = info_json().replace(
            r#""event": "file.info_updated""#,
            r#""event": "file.something_new""#,
        );
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(info.event, "file.something_new");
    }

    #[test]
    fn info_debug_masks_signing_secret() {
        let info: Info = serde_json::from_str(info_json()).unwrap();
        let debug = format!("{:?}", info);

        assert!(!debug.contains("s3cr3t"), "secret leaked into {}", debug);
        assert!(debug.contains("<masked>"));
        // the rest is still there to debug with
        assert!(debug.contains("1387"));
        assert!(debug.contains("https://example.com/uploadcare/hook"));
    }

    #[test]
    fn info_debug_keeps_absent_secret_absent() {
        let json =
            info_json().replace(r#""signing_secret": "s3cr3t""#, r#""signing_secret": null"#);
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        assert!(format!("{:?}", info).contains("signing_secret: None"));
    }

    #[test]
    fn create_params_serialize_new_events() {
        let params = CreateParams {
            event: Event::FileInfoUpdated,
            target_url: "https://example.com/uploadcare/hook".to_string(),
            signing_secret: Some("s3cr3t".to_string()),
            is_active: Some(true),
            version: Some(Version::V07),
        };

        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            serde_json::json!({
                "event": "file.info_updated",
                "target_url": "https://example.com/uploadcare/hook",
                "signing_secret": "s3cr3t",
                "is_active": true,
                "version": "0.7",
            }),
        );
    }

    #[test]
    fn create_params_event_names() {
        let name = |event: Event| {
            serde_json::to_value(CreateParams {
                event,
                target_url: "https://example.com/hook".to_string(),
                signing_secret: None,
                is_active: None,
                version: None,
            })
            .unwrap()["event"]
                .clone()
        };

        assert_eq!(
            name(Event::FileUploaded),
            serde_json::json!("file.uploaded")
        );
        assert_eq!(
            name(Event::FileInfected),
            serde_json::json!("file.infected")
        );
        assert_eq!(name(Event::FileStored), serde_json::json!("file.stored"));
        assert_eq!(name(Event::FileDeleted), serde_json::json!("file.deleted"));
        assert_eq!(
            name(Event::FileInfoUpdated),
            serde_json::json!("file.info_updated"),
        );
    }

    #[test]
    fn update_params_cannot_carry_a_version() {
        // changing the subscription version is answered with 400, so the field is
        // absent from UpdateParams altogether; this pins that down
        let params = UpdateParams {
            id: 1387,
            event: None,
            target_url: None,
            signing_secret: None,
            is_active: Some(true),
        };
        let value = serde_json::to_value(&params).unwrap();

        assert!(value.get("version").is_none());
        // partial update: only the set fields travel. In particular an unset
        // signing_secret must not be sent as null (that risks wiping the
        // stored secret), and `id` belongs to the request path, not the body.
        assert_eq!(value, serde_json::json!({"is_active": true}));
    }

    #[test]
    fn create_params_omit_unset_fields() {
        let params = CreateParams {
            event: Event::FileUploaded,
            target_url: "https://example.com/hook".to_string(),
            signing_secret: None,
            is_active: None,
            version: Some(Version::V07),
        };

        // unset optional fields are not sent as nulls, the API defaults apply
        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            serde_json::json!({
                "event": "file.uploaded",
                "target_url": "https://example.com/hook",
                "version": "0.7",
            }),
        );
    }
}
