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

use crate::file::{ImageInfo, VideoInfo};
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
        let mut form = Form::new()
            .file(params.name.to_string(), params.path.to_string())?
            .text(
                "UPLOADCARE_STORE",
                if let Some(val) = params.to_store {
                    val
                } else {
                    ToStore::False
                }
                .to_string(),
            );
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
        let mut form = Form::new().text("source_url", params.source_url).text(
            "store",
            if let Some(val) = params.to_store {
                val
            } else {
                ToStore::False
            }
            .to_string(),
        );
        if let Some(val) = params.filename {
            form = form.text("filename", val);
        }
        if let Some(val) = params.check_url_duplicates {
            form = form.text("check_URL_duplicates", val.to_string());
        }
        if let Some(val) = params.save_url_duplicates {
            form = form.text("save_URL_duplicates", val.to_string());
        }
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
            .text(
                "UPLOADCARE_STORE",
                if let Some(val) = params.to_store {
                    val
                } else {
                    ToStore::False
                }
                .to_string(),
            )
            .text("content_type", params.content_type)
            .text("size", params.size.to_string());
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
    /// File storing behaviour.
    pub to_store: Option<ToStore>,
}

/// Parameters for upload from public URL link
pub struct FromUrlParams {
    /// File URL, which should be a public HTTP or HTTPS link
    pub source_url: String,
    /// File storing behaviour.
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
        done: u32,
        /// Total file size in bytes
        total: u32,
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
    pub done: u32,
    /// Same as uuid
    pub file_id: String,
    /// Total is same as size
    pub total: u32,
    /// File size in bytes
    pub size: u32,
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
    pub size: u32,
    /// A file MIME-type
    pub content_type: String,
    /// File storing behaviour.
    pub to_store: Option<ToStore>,
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
