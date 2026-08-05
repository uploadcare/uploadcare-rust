//! Upload module contains all upload related API stuff.
//!
//! Upload API is an addition to the REST API. It provides several ways of uploading
//! files to the Uploadcare servers.
//! Every uploaded file is temporary and subject to be deleted within a 24-hour
//! period. To make any file permanent, you should store or copy it.
//!
//! The package provides uploading files by making requests with payload to
//! the Uploadcare API endpoints. There are two basic upload types:
//!
//! - Direct uploads, a regular upload mode that suits most files less than 100MB
//! in size. You won’t be able to use this mode for larger files.
//!
//! - Multipart uploads, a more sophisticated upload mode supporting any files
//! larger than 10MB and implementing accelerated uploads through
//! a distributed network.

use std::collections::HashMap;
use std::fmt::{self, Debug, Display};

use reqwest::{blocking::multipart::Form, Method, Url};
use serde::Deserialize;

use crate::types::ImageInfo;
use crate::ucare::{upload::Client, upload::Fields, upload::Payload, Result};

/// Service is used to make calls to file API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates new upload service instance
pub fn new_svc(client: &Client) -> Service {
    Service { client }
}

impl Service<'_> {
    /// Uploads a file and return its unique id (uuid). Comply with the RFC7578 standard.
    /// Resulting HashMap holds filenames as keys and their ids are values.
    pub fn file(&self, params: FileParams) -> Result<HashMap<String, String>> {
        let mut form = Form::new().file(params.name.to_string(), params.path.to_string())?;
        if let Some(val) = params.to_store {
            form = form.text("UPLOADCARE_STORE", val.to_string());
        }
        form = add_metadata_tags(form, params.metadata, params.tags);
        form = add_signature_expire(&(*self.client.auth_fields)(), form);

        self.client.call::<String, HashMap<String, String>>(
            Method::POST,
            format!("/base/"),
            None,
            Some(Payload::Form(form)),
        )
    }

    /// Uploads file by its public URL.
    pub fn from_url(&self, params: FromUrlParams) -> Result<FromUrlData> {
        let mut form = Form::new().text("source_url", params.source_url);
        if let Some(val) = params.to_store {
            form = form.text("store", val.to_string());
        }
        if let Some(val) = params.filename {
            form = form.text("filename", val);
        }
        if let Some(val) = params.check_url_duplicates {
            form = form.text("check_URL_duplicates", val.to_string());
        }
        if let Some(val) = params.save_url_duplicates {
            form = form.text("save_URL_duplicates", val.to_string());
        }
        // this endpoint takes metadata but no tags
        form = add_metadata_tags(form, params.metadata, None);
        form = add_signature_expire(&(*self.client.auth_fields)(), form);

        self.client.call::<String, FromUrlData>(
            Method::POST,
            format!("/from_url/"),
            None,
            Some(Payload::Form(form)),
        )
    }

    /// Check the status of a file uploaded from URL.
    pub fn from_url_status(&self, token: &str) -> Result<FromUrlStatusData> {
        self.client.call::<String, FromUrlStatusData>(
            Method::GET,
            format!("/from_url/status/?token={}", token),
            None,
            None,
        )
    }

    /// Returns uploading file info.
    pub fn file_info(&self, file_id: &str) -> Result<FileInfo> {
        let fields = (*self.client.auth_fields)();
        self.client.call::<String, FileInfo>(
            Method::GET,
            format!("/info/?pub_key={}&file_id={}", fields.pub_key, file_id),
            None,
            None,
        )
    }

    /// Creates files group from a set of files by using their IDs with
    /// or without applied CDN media processing operations.
    ///
    /// Example:
    ///   [
    ///      "d6d34fa9-addd-472c-868d-2e5c105f9fcd",
    ///      "b1026315-8116-4632-8364-607e64fca723/-/resize/x800/",
    ///   ]
    pub fn create_group(&self, ids: &[&str]) -> Result<GroupInfo> {
        let mut form = Form::new();
        for (pos, id) in ids.iter().enumerate() {
            form = form.text(
                ("files[".to_string() + pos.to_string().as_str() + "]").to_string(),
                id.to_string(),
            );
        }
        form = add_signature_expire(&(*self.client.auth_fields)(), form);

        self.client.call::<String, GroupInfo>(
            Method::POST,
            format!("/group/"),
            None,
            Some(Payload::Form(form)),
        )
    }

    /// Returns group specific info.
    ///
    /// GroupID look like UUID~N, for example:
    ///   "d52d7136-a2e5-4338-9f45-affbf83b857d~2"
    pub fn group_info(&self, group_id: &str) -> Result<GroupInfo> {
        let fields = (*self.client.auth_fields)();
        self.client.call::<String, GroupInfo>(
            Method::GET,
            format!(
                "/group/info/?pub_key={}&group_id={}",
                fields.pub_key, group_id,
            ),
            None,
            None,
        )
    }

    /// Multipart upload is useful when you are dealing with file larger than
    /// 100MB or explicitly want to use accelerated uploads.
    /// Another benefit is your file will go straight to AWS S3 bypassing our upload
    /// instances thus quickly becoming available for further use.
    /// Note, there also exists a minimum file size to use with Multipart Uploads, 10MB.
    /// Trying to use Multipart upload with a smaller file will result in an error.
    pub fn multipart_start(&self, params: MultipartParams) -> Result<MultipartData> {
        let mut form = Form::new()
            .text("filename", params.filename)
            .text("content_type", params.content_type)
            .text("size", params.size.to_string());
        if let Some(val) = params.to_store {
            form = form.text("UPLOADCARE_STORE", val.to_string());
        }
        if let Some(val) = params.part_size {
            form = form.text("part_size", val.to_string());
        }
        form = add_metadata_tags(form, params.metadata, params.tags);
        form = add_signature_expire(&(*self.client.auth_fields)(), form);

        self.client.call::<String, MultipartData>(
            Method::POST,
            format!("/multipart/start/"),
            None,
            Some(Payload::Form(form)),
        )
    }

    /// The second phase is about uploading file parts to the provided URLs. Each uploaded part
    /// should be 5MB (5242880 bytes) in size except for the last one that can be smaller. You
    /// can upload file parts in parallel provided the byte order stays unchanged. Make sure to
    /// define Content-Type header for your data.
    pub fn upload_part(&self, url: &str, data: Vec<u8>) -> Result<()> {
        self.client
            .call_url::<()>(Method::PUT, Url::parse(url)?, Some(Payload::Raw(data)))
    }

    /// Complete multipart upload transaction when all file parts are uploaded
    pub fn multipart_complete(&self, uuid: String) -> Result<FileInfo> {
        let mut form = Form::new().text("uuid", uuid);
        form = add_signature_expire(&(*self.client.auth_fields)(), form);

        self.client.call::<String, FileInfo>(
            Method::POST,
            format!("/multipart/complete/"),
            None,
            Some(Payload::Form(form)),
        )
    }
}

/// Holds all possible params for the file upload
#[derive(Default)]
pub struct FileParams {
    /// Path of the file to upload.
    ///
    /// It must be smaller than 100MB.
    /// An attempt of reading a larger file raises a 413 error with the
    /// respective description. If you want to upload larger files, please
    /// use multipart upload API methods.
    pub path: String,
    /// Uploaded file name
    pub name: String,
    /// File storing behaviour. Left to the API default when None.
    pub to_store: Option<ToStore>,
    /// Arbitrary metadata to attach to the file, sent as `metadata[key]` fields.
    ///
    /// Keys are limited to 64 characters and values to non empty strings of up to
    /// 512, same as the file metadata of the REST API. Values are strings only:
    /// numbers, booleans and nested objects cannot be stored.
    pub metadata: HashMap<String, String>,
    /// Tags to attach to the file.
    ///
    /// Up to 50 of them, each up to 100 characters of lowercase latin letters,
    /// digits, `-`, `_` and `.`. The API lowercases, trims and deduplicates them, so
    /// what comes back may differ from what was sent; this crate passes the values
    /// through as they are rather than normalizing locally.
    ///
    /// An empty vector is treated the same as `None` and sends no field at all.
    pub tags: Option<Vec<String>>,
}

/// Parameters for upload from public URL link
pub struct FromUrlParams {
    /// File URL, which should be a public HTTP or HTTPS link
    pub source_url: String,
    /// File storing behaviour. Left to the API default when None.
    pub to_store: Option<ToStore>,
    /// The name for a file uploaded from URL. If not defined, the filename is obtained from
    /// either response headers or a source URL
    pub filename: Option<String>,
    /// Specify to run the duplicate check and provide the immediate-download behavior
    pub check_url_duplicates: Option<UrlDuplicates>,
    /// Specify to run The save/update URL behavior. The parameter can be used if you believe a
    /// `source_url` will be used more than once. If you don’t explicitly defined, it is by
    /// default set to the value of `check_url_duplicates`.
    pub save_url_duplicates: Option<UrlDuplicates>,
    /// Arbitrary metadata to attach to the file, sent as `metadata[key]` fields.
    /// See [`FileParams::metadata`].
    ///
    /// Unlike the direct and the multipart upload, this endpoint takes no tags.
    pub metadata: HashMap<String, String>,
}

/// Holds data returned by `from_url`
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FromUrlData {
    /// Token
    #[serde(rename = "token")]
    Token(FileToken),
    /// File info
    #[serde(rename = "file_info")]
    FileInfo(FileInfo),
}

impl Default for FromUrlData {
    fn default() -> Self {
        FromUrlData::Token(FileToken::default())
    }
}

/// Respose for the `FromUrlData::Token`
#[derive(Debug, Deserialize, Default)]
pub struct FileToken {
    /// Value: "token"
    #[serde(rename = "type")]
    pub data_type: String,
    /// A token to identify a file for the upload status request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Holds the response returned by `from_url_status`
#[derive(Debug, Deserialize)]
#[serde(tag = "status")]
pub enum FromUrlStatusData {
    /// Success
    #[serde(rename = "success")]
    Success(FileInfo),
    /// Still in progress
    #[serde(rename = "progress")]
    Progress {
        /// Currently uploaded file size in bytes
        done: u64,
        /// Total file size in bytes
        total: u64,
    },
    /// File upload error
    #[serde(rename = "error")]
    Error {
        /// Error description
        error: String,
    },
    /// Unknown
    #[serde(rename = "unknown")]
    Unknown,
    /// Waiting
    #[serde(rename = "waiting")]
    Waiting,
}

impl Default for FromUrlStatusData {
    fn default() -> Self {
        FromUrlStatusData::Unknown
    }
}

/// Holds file information in the upload context
#[derive(Debug, Deserialize, Default)]
pub struct FileInfo {
    /// True if file is stored
    pub is_stored: bool,
    /// Denotes currently uploaded file size in bytes
    pub done: u64,
    /// Same as uuid
    pub file_id: String,
    /// Total is same as size
    pub total: u64,
    /// File size in bytes
    pub size: u64,
    /// File UUID
    pub uuid: String,
    /// If file is an image
    pub is_image: bool,
    /// Sanitized `original_filename
    pub filename: String,
    /// Video metadata
    pub video_info: Option<VideoInfo>,
    /// If file is ready to be used after upload
    pub is_ready: bool,
    /// Original file name taken from uploaded file
    pub original_filename: String,
    /// Image metadata
    pub image_info: Option<ImageInfo>,
    /// File MIME-type.
    pub mime_type: String,
    /// Your custom user bucket on which file are stored.
    /// Only available of you setup foreign storage bucket for your project
    pub s3_bucket: Option<String>,
    /// CDN media transformations applied to the file when its group was created
    pub default_effects: Option<String>,
}

/// Video related information as returned by the Upload API.
///
/// Not to be confused with [`crate::file::VideoInfo`]: the Upload API is versioned
/// separately from the REST API and keeps the pre-v0.7 shape, where `audio` and
/// `video` are single objects rather than lists of streams.
#[derive(Debug, PartialEq, Deserialize)]
pub struct VideoInfo {
    /// Video duration in milliseconds.
    pub duration: Option<f32>,
    /// Video format (MP4 for example).
    pub format: Option<String>,
    /// Video bitrate.
    pub bitrate: Option<f32>,
    /// Audio information
    pub audio: Option<VideoInfoAudio>,
    /// Video stream info
    pub video: Option<VideoInfoVideo>,
}

/// Information about the audio in video
#[derive(Debug, PartialEq, Deserialize)]
pub struct VideoInfoAudio {
    /// Audio stream metadata.
    pub bitrate: Option<f32>,
    /// Audio stream codec.
    pub codec: Option<String>,
    /// Audio stream sample rate.
    pub sample_rate: Option<f32>,
    /// Audio stream number of channels.
    pub channels: Option<String>,
}

/// Video stream info
#[derive(Debug, PartialEq, Deserialize)]
pub struct VideoInfoVideo {
    /// Video stream image height.
    pub height: Option<f32>,
    /// Video stream image width.
    pub width: Option<f32>,
    /// Video stream frame rate.
    pub frame_rate: Option<f32>,
    /// Video stream bitrate.
    pub bitrate: Option<f32>,
    /// Video stream codec.
    pub codec: Option<String>,
}

/// Group information
#[derive(Debug, Deserialize, Default)]
pub struct GroupInfo {
    /// When group was created
    pub datetime_created: String,
    /// When group was stored
    pub datetime_stored: Option<String>,
    /// Number of files in the group
    #[serde(rename = "files_count")]
    pub file_count: u32,
    /// CDN URL of the group
    pub cdn_url: String,
    /// Files list
    pub files: Option<Vec<FileInfo>>,
    /// Group API url to get this info
    pub url: String,
    /// Group ID
    pub id: String,
}

/// Params for starting multipart upload
#[derive(Debug, Default)]
pub struct MultipartParams {
    /// Original file name
    pub filename: String,
    /// Precise file size in bytes. Should not exceed your project file size cap.
    pub size: u64,
    /// A file MIME-type
    pub content_type: String,
    /// File storing behaviour. Left to the API default when None.
    pub to_store: Option<ToStore>,
    /// Expected size of a single part in bytes.
    ///
    /// Left to the API default of 5242880 (5 MiB) when None. Worth raising for files
    /// over a gigabyte, otherwise the part count — and with it the number of
    /// presigned urls in the response — grows into the thousands.
    ///
    /// Whatever is chosen here decides how [`Service::upload_part`] has to slice the
    /// file: every part but the last one MUST be exactly this size.
    pub part_size: Option<u64>,
    /// Arbitrary metadata to attach to the file, sent as `metadata[key]` fields.
    /// See [`FileParams::metadata`].
    pub metadata: HashMap<String, String>,
    /// Tags to attach to the file. See [`FileParams::tags`].
    pub tags: Option<Vec<String>>,
}

/// Response for starting multipart upload
#[derive(Default, Debug, Deserialize)]
pub struct MultipartData {
    /// Array of presigned-url strings    
    pub parts: Vec<String>,
    /// Uploaded file UUID
    pub uuid: String,
}

/// Upload status
pub enum UploadStatus {
    /// success
    Success,
    /// progress
    InProgress,
    /// error
    Error,
    /// waiting
    Waiting,
    /// unknown
    Unknown,
}

impl Display for UploadStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            UploadStatus::Success => "success",
            UploadStatus::InProgress => "progress",
            UploadStatus::Error => "error",
            UploadStatus::Waiting => "waiting",
            UploadStatus::Unknown => "unknown",
        };

        write!(f, "{}", val)
    }
}

/// Sets the file storing behaviour
///
/// Leaving the parameter unset on the params structs sends no field at all, which
/// lets the API apply its own default. That default is `Auto` for projects registered
/// after February 12, 2024 and `False` for the older ones, so it is worth being
/// explicit whenever the behaviour matters.
pub enum ToStore {
    /// True
    True,
    /// False
    False,
    /// Auto
    Auto,
}

impl Display for ToStore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            ToStore::True => "1",
            ToStore::False => "0",
            ToStore::Auto => "auto",
        };

        write!(f, "{}", val)
    }
}

impl Debug for ToStore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ToStore {}", self)
    }
}

/// Used for FormUrlParams
pub enum UrlDuplicates {
    /// True
    True,
    /// False
    False,
}

impl Display for UrlDuplicates {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            UrlDuplicates::True => "1",
            UrlDuplicates::False => "0",
        };

        write!(f, "{}", val)
    }
}

/// Adds the file metadata and tags fields to an upload form.
///
/// Shared by the direct and the multipart upload, both of which accept them in
/// exactly the same shape.
fn add_metadata_tags(
    mut form: Form,
    metadata: HashMap<String, String>,
    tags: Option<Vec<String>>,
) -> Form {
    for (key, value) in metadata {
        form = form.text(metadata_field(key.as_str()), value);
    }
    if let Some(value) = encode_tags(tags) {
        form = form.text("tags", value);
    }

    form
}

/// Builds the form field name for a metadata key.
fn metadata_field(key: &str) -> String {
    format!("metadata[{}]", key)
}

/// Encodes tags the way the API expects them: one comma separated field rather than
/// a repeated one. `None` when there is nothing to send.
fn encode_tags(tags: Option<Vec<String>>) -> Option<String> {
    match tags {
        None => None,
        Some(tags) if tags.is_empty() => None,
        Some(tags) => Some(tags.join(",")),
    }
}

fn add_signature_expire(auth_fields: &Fields, form: Form) -> Form {
    let form = form
        .text("UPLOADCARE_PUB_KEY", auth_fields.pub_key.to_string())
        .text("pub_key", auth_fields.pub_key.to_string());
    if let None = auth_fields.signature {
        return form;
    }
    form.text(
        "signature",
        auth_fields.signature.as_ref().unwrap().to_string(),
    )
    .text("expire", auth_fields.expire.as_ref().unwrap().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_field_names() {
        assert_eq!(metadata_field("subsystem"), "metadata[subsystem]");
        // any unicode letter is a valid key character, the brackets are all we add
        assert_eq!(metadata_field("отдел"), "metadata[отдел]");
    }

    #[test]
    fn tags_are_comma_separated() {
        assert_eq!(
            encode_tags(Some(vec!["invoice".to_string(), "2026".to_string()])),
            Some("invoice,2026".to_string()),
        );
        assert_eq!(
            encode_tags(Some(vec!["invoice".to_string()])),
            Some("invoice".to_string()),
        );
    }

    #[test]
    fn tags_send_no_field_when_there_is_nothing_to_send() {
        assert_eq!(encode_tags(None), None);
        assert_eq!(encode_tags(Some(vec![])), None);
    }

    #[test]
    fn tags_are_passed_through_unnormalized() {
        // the API lowercases, trims and deduplicates; doing it here too would only
        // make the crate disagree with the service on the details
        assert_eq!(
            encode_tags(Some(vec!["Invoice".to_string(), "invoice".to_string()])),
            Some("Invoice,invoice".to_string()),
        );
    }

    #[test]
    fn file_params_default_carries_no_metadata_or_tags() {
        let params = FileParams::default();

        assert!(params.metadata.is_empty());
        assert_eq!(params.tags, None);
    }

    #[test]
    fn multipart_params_default_leaves_part_size_to_the_api() {
        let params = MultipartParams::default();

        assert_eq!(params.part_size, None);
        assert!(params.metadata.is_empty());
        assert_eq!(params.tags, None);
    }
}
