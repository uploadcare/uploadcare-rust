//! Holds all primitives and logic related to file conversion.
//!
//! Uploadcare allows converting documents to the following target formats:
//! DOC, DOCX, XLS, XLSX, ODT, ODS, RTF, TXT, PDF, JPG, PNG.

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
    pub fn document_status(&self, token: i32) -> Result<StatusResult> {
        self.client.call::<String, String, StatusResult>(
            Method::GET,
            format!("/convert/document/status/{}/", token),
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
    pub fn video_status(&self, token: i32) -> Result<StatusResult> {
        self.client.call::<String, String, StatusResult>(
            Method::POST,
            format!("convert/video/status/{}/", token),
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
    /// file converted to. The supported values for :target-format are: doc,
    /// docx, xls, xlsx, odt, ods, rtf, txt, pdf (default), jpg, png. In case
    /// the /format/ operation was not found, your input document will be
    /// converted to pdf. Note, when converting multi-page documents to image
    /// formats (jpg or png), your output will be a zip archive holding a
    /// number of images corresponding to the input page count.
    ///   /page/:number/ converts a single page of a multi-paged document to
    /// either jpg or png. The method will not work for any other target
    /// formats. :number stands for the one-based number of a page to convert.
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
    pub token: Option<i32>,
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
