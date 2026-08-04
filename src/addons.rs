//! Holds all primitives and logic related to Add-Ons.
//!
//! Add-Ons are applications that process an already uploaded file: virus scanning,
//! object recognition, background removal and so on. Available since APIv0.7 only.
//!
//! Processing is asynchronous and takes two steps. [`Service::execute`] runs all the
//! checks synchronously, queues the job and returns a `request_id` right away — a
//! successful response means "accepted", not "finished". [`Service::status`] is then
//! polled with that `request_id` until the status becomes
//! [`Status::Done`] or [`Status::Error`].
//!
//! [`Service::execute_and_wait`] wraps the two into a single call with an explicit
//! timeout, which is usually what you want.
//!
//! Where the actual output lands depends on the application. Most of them return no
//! `result` at all and write into the file properties instead, readable through
//! [`crate::file::Info::appdata`] — `{"status": "done"}` with no `result` is a normal
//! success, not a parsing problem. `remove_bg` is the exception: it creates a new
//! file and returns its id in `result`.

use std::cmp::min;
use std::thread;
use std::time::{Duration, Instant};

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::ucare::{encode_json, rest::Client, Result};

/// Delay before the first status poll.
const FIRST_POLL_DELAY: Duration = Duration::from_secs(1);
/// Upper bound for the status poll interval.
const MAX_POLL_DELAY: Duration = Duration::from_secs(5);

/// Service is used to make calls to the Add-Ons API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the addons service
pub fn new_svc(client: &Client) -> Service {
    Service { client }
}

impl Service<'_> {
    /// Starts processing a file by an application.
    ///
    /// `application_id` is a plain string on purpose: the set of applications grows
    /// without an API version bump, so it is not modelled as an enum. Known values at
    /// the time of writing are `uc_clamav_virus_scan`, `aws_rekognition_detect_labels`,
    /// `aws_rekognition_detect_moderation_labels` and `remove_bg`.
    ///
    /// A successful call only means the job was accepted. Notable errors:
    ///
    /// - `ErrValue::Conflict` — the same application is already processing this file.
    ///   Treat it as "already running", not as "retry in a second": if the previous
    ///   run died without updating its status, the pair stays locked for up to 24
    ///   hours.
    /// - `ErrValue::Forbidden` — the application is disabled for the project.
    ///   Permissions are per application, so this says nothing about the others.
    /// - `ErrValue::NotFound` — unknown `application_id`, or the file is not in the
    ///   project. The two are indistinguishable by status code.
    /// - `ErrValue::TooManyRequests` — the launch rate limit, 10 to 600 per minute
    ///   depending on the project plan. Polling is not affected by it.
    ///
    /// Calls are not idempotent: every successful one starts a new job with a new
    /// `request_id`, and for `remove_bg` that means another new file. Do not blindly
    /// retry a call whose response was lost.
    pub fn execute(&self, application_id: &str, params: ExecuteParams) -> Result<Execution> {
        let json = encode_json(&params)?;

        self.client.call::<String, Vec<u8>, Execution>(
            Method::POST,
            format!("/addons/{}/execute/", application_id),
            None,
            Some(json),
        )
    }

    /// Gets the status of a started execution.
    ///
    /// The state is addressed by `request_id` alone — `application_id` does not take
    /// part in the lookup, but still has to be an existing one, otherwise the API
    /// answers `404`. Pass the same one that was used to start the job.
    pub fn status(&self, application_id: &str, request_id: &str) -> Result<StatusInfo> {
        self.client.call::<String, String, StatusInfo>(
            Method::GET,
            format!("/addons/{}/execute/status/", application_id),
            Some(format!("request_id={}", request_id)),
            None,
        )
    }

    /// Starts processing a file and polls the status until it settles or `timeout`
    /// elapses.
    ///
    /// Blocks the calling thread. The first poll happens after a second, then the
    /// interval grows by half up to 5 seconds — the API has no recommended schedule,
    /// and polling more often than once a second buys nothing.
    ///
    /// `timeout` has to be generous: a virus scan of a half gigabyte file can take
    /// tens of minutes. On expiry [`Outcome::Timeout`] is returned with the
    /// `request_id`, so polling can be resumed later through [`Service::wait`]
    /// instead of starting the job over.
    ///
    /// ```rust,ignore
    /// # use std::time::Duration;
    /// # use ucare::addons;
    ///
    /// let params = addons::ExecuteParams::with_params(
    ///     "1bac376c-aa7e-4356-861b-dd2657b5bfd1",
    ///     &addons::RemoveBgParams {
    ///         crop: Some(true),
    ///         foreground_type: Some(addons::ForegroundType::Person),
    ///         ..Default::default()
    ///     },
    /// )?;
    ///
    /// match addons_svc.execute_and_wait("remove_bg", params, Duration::from_secs(300))? {
    ///     addons::Outcome::Done { result, .. } => println!("done: {:?}", result),
    ///     addons::Outcome::Error { details, .. } => println!("failed: {:?}", details),
    ///     addons::Outcome::Unknown { .. } => println!("state expired or never existed"),
    ///     addons::Outcome::Timeout { request_id } => println!("still running: {}", request_id),
    /// }
    /// ```
    pub fn execute_and_wait(
        &self,
        application_id: &str,
        params: ExecuteParams,
        timeout: Duration,
    ) -> Result<Outcome> {
        let request_id = self.execute(application_id, params)?.request_id;

        self.wait(application_id, request_id.as_str(), timeout)
    }

    /// Polls the status of an already started execution until it settles or `timeout`
    /// elapses. See [`Service::execute_and_wait`] for the polling schedule.
    pub fn wait(
        &self,
        application_id: &str,
        request_id: &str,
        timeout: Duration,
    ) -> Result<Outcome> {
        let started = Instant::now();
        let mut delay = FIRST_POLL_DELAY;

        loop {
            // sleeping before the first poll on purpose: the job has just been
            // queued, an immediate request can only answer in_progress
            let left = timeout.saturating_sub(started.elapsed());
            if left.is_zero() {
                return Ok(Outcome::Timeout {
                    request_id: request_id.to_string(),
                });
            }
            thread::sleep(min(delay, left));

            let info = self.status(application_id, request_id)?;
            match info.status {
                Status::Done => {
                    return Ok(Outcome::Done {
                        request_id: request_id.to_string(),
                        result: info.result,
                    })
                }
                Status::Error => {
                    return Ok(Outcome::Error {
                        request_id: request_id.to_string(),
                        details: info.details,
                    })
                }
                Status::Unknown => {
                    return Ok(Outcome::Unknown {
                        request_id: request_id.to_string(),
                    })
                }
                Status::InProgress => (),
            }

            delay = min(delay * 3 / 2, MAX_POLL_DELAY);
        }
    }
}

/// Holds all possible params for the execute method
#[derive(Debug, Serialize)]
pub struct ExecuteParams {
    /// UUID of the file to process. MUST belong to the project the request is made
    /// on behalf of.
    pub target: String,
    /// Application specific params. The set of them depends on the application.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl ExecuteParams {
    /// Params for an application that takes no options, object recognition
    /// for example.
    pub fn new(target: &str) -> Self {
        ExecuteParams {
            target: target.to_string(),
            params: None,
        }
    }

    /// Params for an application that takes options, [`RemoveBgParams`] for example.
    ///
    /// Accepts anything serializable, including a `serde_json::json!` literal for an
    /// application this module has no typed params for. Prefer the typed structs
    /// where they exist: the API silently drops unknown keys, so a typo in a hand
    /// written literal is never reported — the request succeeds and the option is
    /// just not applied.
    pub fn with_params<T>(target: &str, params: &T) -> Result<Self>
    where
        T: ?Sized + Serialize,
    {
        Ok(ExecuteParams {
            target: target.to_string(),
            params: Some(serde_json::to_value(params)?),
        })
    }
}

/// Holds the execute response data
#[derive(Debug, Deserialize)]
pub struct Execution {
    /// Identifier of the started execution, generated by the API. Used to poll
    /// the status.
    pub request_id: String,
}

/// Holds the execution status response data
#[derive(Debug, Deserialize)]
pub struct StatusInfo {
    /// Current state of the execution.
    pub status: Status,
    /// Application output.
    ///
    /// Present only for [`Status::Done`] and only for applications that return one.
    /// Most of them do not, so `None` here is not an error — read their output from
    /// the file properties instead, see the module docs.
    pub result: Option<serde_json::Value>,
    /// Machine readable failure description.
    ///
    /// Present only for [`Status::Error`] and only when the failure has one, so
    /// `None` here is not an error either.
    pub details: Option<Details>,
}

/// State of an Add-On execution
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize)]
#[non_exhaustive]
pub enum Status {
    /// Accepted and running.
    #[serde(rename = "in_progress")]
    InProgress,
    /// Finished successfully.
    #[serde(rename = "done")]
    Done,
    /// Finished unsuccessfully.
    #[serde(rename = "error")]
    Error,
    /// The API knows nothing about this `request_id`: it never existed, or the state
    /// is older than the 24 hours it is kept for. The two are indistinguishable, so
    /// after a previously seen [`Status::InProgress`] this almost certainly means
    /// expiry rather than a bad `request_id`.
    #[serde(rename = "unknown")]
    Unknown,
}

/// Machine readable description of a failed execution
#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Details {
    /// Failure code, `unknown_foreground` for example.
    pub code: Option<String>,
    /// Human readable failure title.
    pub title: Option<String>,
}

/// Settled state of an Add-On execution, as reported by
/// [`Service::execute_and_wait`] and [`Service::wait`]
#[derive(Debug)]
pub enum Outcome {
    /// Finished successfully.
    Done {
        /// Identifier of the execution.
        request_id: String,
        /// Application output, `None` for applications that return none.
        result: Option<serde_json::Value>,
    },
    /// Finished unsuccessfully.
    Error {
        /// Identifier of the execution.
        request_id: String,
        /// Failure description, `None` when the failure has no structured one.
        details: Option<Details>,
    },
    /// The API knows nothing about the execution, see [`Status::Unknown`].
    Unknown {
        /// Identifier of the execution.
        request_id: String,
    },
    /// The timeout elapsed while the execution was still in progress. Polling can be
    /// resumed with [`Service::wait`], the job itself is not affected.
    Timeout {
        /// Identifier of the execution.
        request_id: String,
    },
}

/// Params of the `uc_clamav_virus_scan` application
///
/// Applies to files of any type, up to 512 MiB. Returns no `result`: the verdict is
/// written into the file properties as `{"infected": false}` or
/// `{"infected": true, "infected_with": "..."}`.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct VirusScanParams {
    /// Whether to delete the file if an infection is found. When unset the project
    /// setting applies.
    ///
    /// Note that a successful scan does not guarantee the source file still exists:
    /// with deletion on, an infected one is gone and later requests for it answer
    /// `404`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purge_infected: Option<bool>,
}

/// Params of the `remove_bg` application
///
/// Images only. Unlike the other applications it creates a new file and returns its
/// id as `{"file_id": "<uuid>"}` in the execution result; the source file is left
/// untouched.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct RemoveBgParams {
    /// Whether to crop off all empty regions. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop: Option<bool>,
    /// Margin around the cropped subject, absolute (`30px`) or relative (`10%`).
    /// Defaults to 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_margin: Option<String>,
    /// Scale of the subject relative to the total image size, `50%` for example.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<String>,
    /// Background color as hex without the leading hash: 3, 4, 6 or 8 characters.
    /// The 4 and 8 character forms carry transparency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<String>,
    /// Whether to add an artificial shadow. Not supported for every subject type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_shadow: Option<bool>,
    /// Foreground subject type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub foreground_type: Option<ForegroundType>,
    /// Classification level of the foreground type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_level: Option<TypeLevel>,
    /// Whether to allow semi transparent regions in the result. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semitransparency: Option<bool>,
    /// Whether to return the ready image or just the transparency mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Channels>,
    /// Region of interest as `x1 y1 x2 y2`. All four values MUST share the unit,
    /// either all in `%` or all in `px`: `0% 0% 100px 100px` is rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roi: Option<String>,
    /// Subject position on the canvas: `original`, `center`, a single percentage or
    /// two of them as `horizontal vertical`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}

/// Foreground subject type for the `remove_bg` application
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[non_exhaustive]
pub enum ForegroundType {
    /// "auto"
    #[serde(rename = "auto")]
    Auto,
    /// "person"
    #[serde(rename = "person")]
    Person,
    /// "product"
    #[serde(rename = "product")]
    Product,
    /// "car"
    #[serde(rename = "car")]
    Car,
}

/// Classification level of the foreground type for the `remove_bg` application
///
/// Serialized as a string, not a number, which is what the API expects.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[non_exhaustive]
pub enum TypeLevel {
    /// "1"
    #[serde(rename = "1")]
    One,
    /// "2"
    #[serde(rename = "2")]
    Two,
    /// "none"
    #[serde(rename = "none")]
    None,
    /// "latest"
    #[serde(rename = "latest")]
    Latest,
}

/// What the `remove_bg` application should return
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[non_exhaustive]
pub enum Channels {
    /// "rgba", the ready image
    #[serde(rename = "rgba")]
    Rgba,
    /// "alpha", the transparency mask only
    #[serde(rename = "alpha")]
    Alpha,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_params_without_application_params() {
        let params = ExecuteParams::new("1bac376c-aa7e-4356-861b-dd2657b5bfd1");

        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            serde_json::json!({"target": "1bac376c-aa7e-4356-861b-dd2657b5bfd1"}),
        );
    }

    #[test]
    fn execute_params_with_typed_application_params() {
        let params = ExecuteParams::with_params(
            "1bac376c-aa7e-4356-861b-dd2657b5bfd1",
            &RemoveBgParams {
                crop: Some(true),
                foreground_type: Some(ForegroundType::Person),
                bg_color: Some("81d4fa".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            serde_json::json!({
                "target": "1bac376c-aa7e-4356-861b-dd2657b5bfd1",
                "params": {"crop": true, "type": "person", "bg_color": "81d4fa"},
            }),
        );
    }

    #[test]
    fn remove_bg_type_level_is_a_string() {
        let params = RemoveBgParams {
            type_level: Some(TypeLevel::Two),
            channels: Some(Channels::Alpha),
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            serde_json::json!({"type_level": "2", "channels": "alpha"}),
        );
    }

    #[test]
    fn virus_scan_params_are_omitted_when_unset() {
        assert_eq!(
            serde_json::to_value(VirusScanParams::default()).unwrap(),
            serde_json::json!({}),
        );
        assert_eq!(
            serde_json::to_value(VirusScanParams {
                purge_infected: Some(true),
            })
            .unwrap(),
            serde_json::json!({"purge_infected": true}),
        );
    }

    #[test]
    fn execute_params_accept_a_raw_json_literal() {
        // escape hatch for an application with no typed params here
        let params = ExecuteParams::with_params(
            "1bac376c-aa7e-4356-861b-dd2657b5bfd1",
            &serde_json::json!({"something_new": 1}),
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            serde_json::json!({
                "target": "1bac376c-aa7e-4356-861b-dd2657b5bfd1",
                "params": {"something_new": 1},
            }),
        );
    }

    #[test]
    fn execution_deserializes() {
        let json = r#"{"request_id": "9b27ff0b-b1c3-4c1f-9a4a-5bb5e5d8e4c2"}"#;
        let execution: Execution = serde_json::from_str(json).unwrap();

        assert_eq!(execution.request_id, "9b27ff0b-b1c3-4c1f-9a4a-5bb5e5d8e4c2");
    }

    #[test]
    fn status_in_progress() {
        let info: StatusInfo = serde_json::from_str(r#"{"status": "in_progress"}"#).unwrap();

        assert_eq!(info.status, Status::InProgress);
        assert!(info.result.is_none());
        assert!(info.details.is_none());
    }

    #[test]
    fn status_done_with_result() {
        let json = r#"{
            "status": "done",
            "result": {"file_id": "b0ea3b6f-0e5c-4a2d-9c65-8a2a6f8bd0e1"}
        }"#;
        let info: StatusInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.status, Status::Done);
        assert_eq!(
            info.result.unwrap()["file_id"],
            serde_json::json!("b0ea3b6f-0e5c-4a2d-9c65-8a2a6f8bd0e1"),
        );
    }

    #[test]
    fn status_done_without_result() {
        // how success looks for every application but remove_bg, must not be an error
        let info: StatusInfo = serde_json::from_str(r#"{"status": "done"}"#).unwrap();

        assert_eq!(info.status, Status::Done);
        assert!(info.result.is_none());
    }

    #[test]
    fn status_error_with_details() {
        let json = r#"{
            "status": "error",
            "details": {
                "code": "unknown_foreground",
                "title": "Could not identify foreground in image"
            }
        }"#;
        let info: StatusInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.status, Status::Error);
        assert_eq!(
            info.details,
            Some(Details {
                code: Some("unknown_foreground".to_string()),
                title: Some("Could not identify foreground in image".to_string()),
            }),
        );
    }

    #[test]
    fn status_error_without_details() {
        let info: StatusInfo = serde_json::from_str(r#"{"status": "error"}"#).unwrap();

        assert_eq!(info.status, Status::Error);
        assert!(info.details.is_none());
    }

    #[test]
    fn status_unknown() {
        let info: StatusInfo = serde_json::from_str(r#"{"status": "unknown"}"#).unwrap();

        assert_eq!(info.status, Status::Unknown);
    }
}
