//! Holds all primitives and logic related to file conversion.
//!
//! Since APIv0.7 the target format is an arbitrary string rather than a value from
//! a fixed list: whether a conversion is possible is decided by the API for every
//! particular source file. Use [`Service::document_info`] to find out what a given
//! document can be converted to instead of keeping a format matrix on the client
//! side.

use std::collections::HashMap;
use std::fmt::Debug;

use reqwest::Method;
use serde::{self, Deserialize, Serialize};

use crate::ucare::{encode_json, rest::Client, Result};

/// Service is used to make calls to conversion API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the conversion service
pub fn new_svc(client: &Client) -> Service {
    Service { client }
}

impl Service<'_> {
    /// Starts document conversion job
    pub fn document(&self, params: JobParams) -> Result<JobResult> {
        let json = encode_json(&params)?;
        self.client.call::<String, Vec<u8>, JobResult>(
            Method::POST,
            format!("/convert/document/"),
            None,
            Some(json),
        )
    }

    /// Gets document conversion job status
    pub fn document_status(&self, token: i64) -> Result<StatusResult> {
        self.client.call::<String, String, StatusResult>(
            Method::GET,
            format!("/convert/document/status/{}/", token),
            None,
            None,
        )
    }

    /// Gets information about a document available for conversion: its source
    /// format, the formats it can be converted to and the groups it has already
    /// been converted into.
    ///
    /// Available since APIv0.7 only. This is the supported way of finding out what
    /// a particular file can be converted to.
    pub fn document_info(&self, file_id: &str) -> Result<DocumentInfo> {
        self.client.call::<String, String, DocumentInfo>(
            Method::GET,
            format!("/convert/document/{}/", file_id),
            None,
            None,
        )
    }

    /// Starts video conversion job
    pub fn video(&self, params: JobParams) -> Result<JobResult> {
        let json = encode_json(&params)?;
        self.client.call::<String, Vec<u8>, JobResult>(
            Method::POST,
            format!("/convert/video"),
            None,
            Some(json),
        )
    }

    /// Gets video conversion job status
    pub fn video_status(&self, token: i64) -> Result<StatusResult> {
        self.client.call::<String, String, StatusResult>(
            Method::GET,
            format!("/convert/video/status/{}/", token),
            None,
            None,
        )
    }
}

/// Conversion job params
#[derive(Debug, Serialize)]
pub struct JobParams {
    /// paths is an array of IDs (UUIDs) of your source documents to convert
    /// together with the specified target format.
    /// Here is how it should be specified:
    ///   :uuid/document/-/format/:target-format/
    ///
    /// You can also provide a complete CDN URL. It can then be used as an
    /// alias to your converted file ID (UUID):
    ///   https://ucarecdn.com/:uuid/document/-/format/:target-format/
    ///
    /// :uuid identifies the source file you want to convert, it should be
    /// followed by /document/, otherwise, your request will return an error.
    /// /-/ is a necessary delimiter that helps our API tell file identifiers
    /// from processing operations.
    ///
    /// The following operations are available during conversion:
    ///   /format/:target-format/ defines the target format you want a source
    /// file converted to. Since APIv0.7 :target-format is an arbitrary string
    /// and no longer a value from a fixed list: whether the source format can
    /// be converted to it is checked by the API for this particular file, at
    /// request validation time. An impossible pair is reported as a bad request
    /// saying "Document conversion from X to Y format is not supported.", and
    /// [`Service::document_info`] is the way to learn the available ones
    /// upfront. In case the /format/ operation was not found, your input
    /// document will be converted to pdf. Note, when converting multi-page
    /// documents to image formats (jpg or png), your output will be a zip
    /// archive holding a number of images corresponding to the input page
    /// count.
    ///   /page/:number/ converts a single page of a multi-paged document to
    /// either jpg or png. The method will not work for any other target
    /// formats. :number stands for the one-based number of a page to convert.
    /// It MUST be the last operation in the chain.
    ///   /dpi/:value/ and /quality/:value/ were added in APIv0.7 and apply to
    /// the jpg target format only, any other one is an error. Both accept a
    /// value from a service defined list rather than an arbitrary number, and
    /// the error message for a rejected one holds the allowed values.
    pub paths: Vec<String>,
    /// Flag indicating if we should store your outputs.
    pub store: Option<ToStore>,
}

/// MUST be either true or false
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub enum ToStore {
    /// True
    #[serde(rename = "true")]
    True,
    /// False
    #[serde(rename = "false")]
    False,
}

/// Conversion job request result
#[derive(Debug, Deserialize)]
pub struct JobResult {
    /// Problems related to your processing job, if any. Key is the path you requested.
    pub problems: Option<HashMap<String, String>>,
    /// Result for each requested path, in case of no errors for that path.
    pub result: Option<Vec<JobInfo>>,
}

/// Conversion job info
#[derive(Debug, Deserialize)]
pub struct JobInfo {
    /// UUID of your converted document
    pub uuid: String,
    /// UUID of a file group with thumbnails for an output video,
    /// based on the `thumbs` operation parameters
    pub thumbnails_group_id: Option<String>,
    /// Source file identifier including a target format, if present
    pub original_source: Option<String>,
    /// Conversion job token that can be used to get a job status
    pub token: Option<i64>,
}

/// Conversion job status request result
#[derive(Debug, Deserialize)]
pub struct StatusResult {
    /// Status holds conversion job status, can be one of the following:
    /// pending    — a source file is being prepared for conversion.
    /// processing — conversion is in progress.
    /// finished   — the conversion is finished.
    /// failed     — we failed to convert the source, see error for details.
    /// canceled   — the conversion was canceled.
    pub status: String,
    /// Conversion error if we were unable to handle your file
    pub error: Option<String>,
    /// Result repeats the contents of your processing output
    pub result: JobInfo,
}

/// Information about a document available for conversion
#[derive(Debug, Deserialize)]
pub struct DocumentInfo {
    /// Error description if the document cannot be handled.
    pub error: Option<String>,
    /// Source document format together with everything it can be
    /// and has already been converted to.
    pub format: Option<DocumentFormat>,
}

/// Source document format
#[derive(Debug, Deserialize)]
pub struct DocumentFormat {
    /// Format name, `docx` for example.
    pub name: Option<String>,
    /// Formats this particular document can be converted to.
    #[serde(default)]
    pub conversion_formats: Vec<ConversionFormat>,
    /// Groups the document has already been converted into,
    /// keyed by the target format.
    #[serde(default)]
    pub converted_groups: HashMap<String, String>,
}

/// A target format available for a document
#[derive(Debug, Deserialize)]
pub struct ConversionFormat {
    /// Format name, `docx` for example.
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_info_token_deserializes_as_i64() {
        let json = r#"{
            "uuid": "a18983d0-b0d7-4c8d-968b-2e6d2e1c3ea1",
            "token": 3000000000
        }"#;
        let info: JobInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.token, Some(3_000_000_000_i64));
    }

    #[test]
    fn document_info_deserializes() {
        let json = r#"{
            "error": null,
            "format": {
                "name": "docx",
                "conversion_formats": [
                    {"name": "pdf"},
                    {"name": "png"},
                    {"name": "txt"}
                ],
                "converted_groups": {
                    "pdf": "badfc9f7-f88f-4921-9cc0-22e2c08aa2da~1",
                    "png": "e4c9d0a3-1b2c-4d5e-8f70-1a2b3c4d5e6f~3"
                }
            }
        }"#;
        let info: DocumentInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.error, None);

        let format = info.format.unwrap();
        assert_eq!(format.name, Some("docx".to_string()));
        assert_eq!(format.conversion_formats.len(), 3);
        assert_eq!(format.conversion_formats[0].name, Some("pdf".to_string()));
        // converted_groups is nested inside format, not at the top level
        assert_eq!(format.converted_groups.len(), 2);
        assert_eq!(
            format.converted_groups.get("pdf"),
            Some(&"badfc9f7-f88f-4921-9cc0-22e2c08aa2da~1".to_string()),
        );
    }

    #[test]
    fn document_info_without_converted_groups() {
        let json = r#"{"format": {"name": "jpeg", "conversion_formats": []}}"#;
        let info: DocumentInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.error, None);

        let format = info.format.unwrap();
        assert!(format.conversion_formats.is_empty());
        assert!(format.converted_groups.is_empty());
    }
}
