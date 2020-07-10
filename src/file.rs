//! Holds all primitives and logic around the file resource.
//!
//! The file resource is intended to handle user-uploaded files and
//! is the main Uploadcare resource.
//!
//! Each of uploaded files has an ID (UUID) that is assigned once and never
//! changes later.

use std::collections::HashMap;
use std::fmt::{self, Debug, Display};

use reqwest::{Method, Url};
use serde::{self, Deserialize, Serialize};
use serde_json;

use crate::ucare::{encode_json, rest::Client, IntoUrlQuery, Result};

/// Service is used to make calls to file API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the file service
pub fn new_svc(client: &Client) -> Service {
    Service { client }
}

impl Service<'_> {
    /// Acquires some file specific info
    pub fn info(&self, file_id: &str) -> Result<Info> {
        self.client.call::<String, String, Info>(
            Method::GET,
            format!("/files/{}/", file_id),
            None,
            None,
        )
    }

    /// Returns a list of files
    ///
    /// ```rust,ignore
    /// # use ucare::file;
    ///
    /// let params = file::ListParams{
    ///     limit: Some(10),
    ///     ordering: Some(file::Ordering::Size),
    ///     from: None,
    /// };
    /// let list = file_svc.list(params)?;
    /// let mut next_page = list.next;
    ///
    /// let mut files = list.results.unwrap();
    /// while let Some(next) = next_page {
    ///     let new_page = file_svc.get_page(&next).unwrap();
    ///     next_page = new_page.next;
    ///     files.extend(new_page.results.unwrap());
    /// }
    ///
    /// for f in files.iter() {
    ///     println!("file: {}", f);
    /// }
    /// ```
    pub fn list(&self, params: ListParams) -> Result<List> {
        self.client.call::<ListParams, String, List>(
            Method::GET,
            format!("/files/"),
            Some(params),
            None,
        )
    }

    /// Gets next page by its url
    pub fn get_page(&self, url: &str) -> Result<List> {
        let url = Url::parse(url)?;
        self.client.call_url::<String, List>(Method::GET, url, None)
    }

    /// Store a single file by its id
    pub fn store(&self, file_id: &str) -> Result<Info> {
        self.client.call::<String, String, Info>(
            Method::PUT,
            format!("/files/{}/storage/", file_id),
            None,
            None,
        )
    }

    /// Used to store multiple files in one go. Up to 100 files are
    /// supported per request.
    pub fn batch_store(&self, file_ids: &[&str]) -> Result<BatchInfo> {
        let json = encode_json(&file_ids)?;
        self.client.call::<String, Vec<u8>, BatchInfo>(
            Method::PUT,
            format!("/files/storage/"),
            None,
            Some(json),
        )
    }

    /// Removes file by its id
    pub fn delete(&self, file_id: &str) -> Result<Info> {
        self.client.call::<String, String, Info>(
            Method::DELETE,
            format!("/files/{}/", file_id),
            None,
            None,
        )
    }

    /// Used to delete multiple files in one go. Up to 100 files are
    /// supported per request.
    pub fn batch_delete(&self, file_ids: &[&str]) -> Result<BatchInfo> {
        let json = encode_json(&file_ids)?;
        self.client.call::<String, Vec<u8>, BatchInfo>(
            Method::DELETE,
            format!("/files/storage/"),
            None,
            Some(json),
        )
    }

    /// Copy is the APIv05 version of the LocalCopy and RemoteCopy, use them instead
    pub fn copy(&self, params: CopyParams) -> Result<LocalCopyInfo> {
        let json = encode_json(&params)?;
        self.client.call::<String, Vec<u8>, LocalCopyInfo>(
            Method::POST,
            format!("/files/"),
            None,
            Some(json),
        )
    }

    /// Used to copy original files or their modified versions to
    /// default storage. Source files MAY either be stored or just uploaded and MUST
    /// NOT be deleted
    pub fn local_copy(&self, mut params: CopyParams) -> Result<LocalCopyInfo> {
        if let None = params.store {
            params.store = Some(ToStore::False);
        }
        if let None = params.make_public {
            params.make_public = Some(MakePublic::True);
        }

        let json = encode_json(&params)?;

        self.client.call::<String, Vec<u8>, LocalCopyInfo>(
            Method::POST,
            format!("/files/local_copy/"),
            None,
            Some(json),
        )
    }

    /// Used to copy original files or their modified versions to a custom
    /// storage. Source files MAY either be stored or just uploaded and MUST NOT be
    /// deleted.
    pub fn remote_copy(&self, mut params: CopyParams) -> Result<RemoteCopyInfo> {
        if let None = params.make_public {
            params.make_public = Some(MakePublic::True);
        }

        let json = encode_json(&params)?;

        self.client.call::<String, Vec<u8>, RemoteCopyInfo>(
            Method::POST,
            format!("/files/remote_copy/"),
            None,
            Some(json),
        )
    }
}

/// Info holds file specific information
#[derive(Debug, Deserialize)]
pub struct Info {
    /// File UUID.
    pub uuid: String,
    /// Date and time when a file was removed, if any.
    pub datetime_removed: Option<String>,
    /// Date and time of the last store request, if any.
    pub datetime_stored: Option<String>,
    /// Date and time when a file was uploaded.
    pub datetime_uploaded: Option<String>,
    /// Image metadata
    pub image_info: Option<ImageInfo>,
    /// Is file is image.
    pub is_image: Option<bool>,
    /// Is file is ready to be used after upload.
    pub is_ready: Option<bool>,
    /// File MIME-type.
    pub mime_type: Option<String>,
    /// Publicly available file CDN URL. Available if a file is not deleted.
    pub original_file_url: Option<String>,
    /// Original file name taken from uploaded file.
    pub original_filename: Option<String>,
    /// File size in bytes.
    pub size: Option<i32>,
    /// API resource URL for a particular file.
    pub url: Option<String>,
    /// Dictionary of other files that has been created using this file as source. Used for video,
    /// document and etc. conversion.
    pub variations: Option<serde_json::Value>,
    /// Video info
    pub video_info: Option<VideoInfo>,
    /// File upload source. This field contains information about from where file was uploaded, for
    /// example: facebook, gdrive, gphotos, etc.
    pub source: Option<String>,
    /// Dictionary of file categories with it\"s confidence.
    pub rekognition_info: Option<HashMap<String, f32>>,
}

/// ImageInfo holds image-specific information
#[derive(Debug, Deserialize)]
pub struct ImageInfo {
    /// Image color mode.
    pub color_mode: Option<ColorMode>,
    /// Image orientation from EXIF.
    pub orientation: Option<i32>,
    /// Image format.
    pub format: Option<String>,
    /// Image sequence
    pub sequence: Option<bool>,
    /// Image height in pixels.
    pub height: Option<i32>,
    /// Image width in pixels.
    pub width: Option<i32>,
    /// Image geo location.
    pub geo_location: Option<ImageInfoGeoLocation>,
    /// Image date and time from EXIF.
    pub datetime_original: Option<String>,
    /// Image DPI for two dimensions.
    pub dpi: Option<Vec<f32>>,
}

/// Image geo location
#[derive(Debug, Deserialize)]
pub struct ImageInfoGeoLocation {
    /// Location latitude.
    pub latitude: Option<f32>,
    /// Location longitude.
    pub longitude: Option<f32>,
}

/// Image color mode.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize)]
pub enum ColorMode {
    /// RGB
    RGB,
    /// RGBA
    RGBA,
    /// RGBa
    RGBa,
    /// RGBX
    RGBX,
    /// L
    L,
    /// LA
    LA,
    /// La
    La,
    /// P
    P,
    /// PA
    PA,
    /// CMYK
    CMYK,
    /// YCbCr
    YCbCr,
    /// HSV
    HSV,
    /// LAB
    LAB,
}

/// Video related information
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

/// Holds all possible params for for the list method
pub struct ListParams {
    /// Is set to true if only include removed files in the response,
    /// otherwise existing files are included. Defaults to false.
    pub removed: Option<bool>,
    /// Is set to true if only include files that were stored.
    /// Set to false to include only temporary files.
    /// The default is unset: both stored and not stored files are returned
    pub stored: Option<bool>,
    /// Specifies preferred amount of files in a list for a single
    /// response. Defaults to 100, while the maximum is 1000
    pub limit: Option<i32>,
    /// Specifies the way files are sorted in a returned list.
    /// By default is set to datetime_uploaded.
    pub ordering: Option<Ordering>,
    /// Specifies a starting point for filtering files.
    /// The value depends on your ordering parameter value.
    pub from: Option<String>,
}

/// Specifies the way files are sorted in a returned list.
/// By default is set to datetime_uploaded.
pub enum Ordering {
    /// "datetime_uploaded"
    DatetimeUploaded,
    /// "-datetime_uploaded"
    DatetimeUploadedNeg,
    /// "size"
    Size,
    /// "-size"
    SizeNeg,
}

impl Display for Ordering {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            Ordering::DatetimeUploaded => "datetime_uploaded",
            Ordering::DatetimeUploadedNeg => "-datetime_uploaded",
            Ordering::Size => "size",
            Ordering::SizeNeg => "-size",
        };

        write!(f, "{}", val)
    }
}

impl IntoUrlQuery for ListParams {
    fn into_query(self) -> String {
        let mut q = String::new();
        q.push_str("removed=");
        if let Some(val) = self.removed {
            q.push_str(val.to_string().as_str());
        } else {
            q.push_str("false");
        }
        q.push('&');

        if let Some(val) = self.stored {
            q.push_str("stored=");
            q.push_str(val.to_string().as_str());
            q.push('&');
        }

        q.push_str("limit=");
        if let Some(val) = self.limit {
            q.push_str(val.to_string().as_str());
        } else {
            q.push_str("1000");
        }
        q.push('&');

        q.push_str("ordering=");
        if let Some(val) = self.ordering {
            q.push_str(val.to_string().as_str());
        } else {
            q.push_str(Ordering::DatetimeUploaded.to_string().as_str());
        }

        if let Some(val) = self.from {
            q.push('&');
            q.push_str("from=");
            q.push_str(val.as_str());
        }

        q
    }
}

/// Holds a list of files
#[derive(Debug, Deserialize)]
pub struct List {
    /// Actual results
    pub results: Option<Vec<Info>>,
    /// Next page URL.
    pub next: Option<String>,
    /// Previous page URL.
    pub previous: Option<String>,
    /// A total number of objects of the queried type. For files, the queried type depends on
    /// the stored and removed query parameters.
    pub total: Option<i32>,
    /// Number of objects per page.
    pub per_page: Option<i32>,
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

/// MUST be either true or false. true to make copied files available via public links,
/// false to reverse the behavior.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub enum MakePublic {
    /// True
    #[serde(rename = "true")]
    True,
    /// False
    #[serde(rename = "false")]
    False,
}

/// The parameter is used to specify file names Uploadcare passes to a custom storage.
/// In case the parameter is omitted, we use pattern of your custom storage.
/// Use any combination of allowed values.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub enum Pattern {
    /// Default
    #[serde(rename = "${default}")]
    Default,
    /// AutoFilename
    #[serde(rename = "${filename} ${effects} ${ext}")]
    AutoFilename,
    /// Effects
    #[serde(rename = "${effects}")]
    Effects,
    /// Filename
    #[serde(rename = "${filename}")]
    Filename,
    /// Uuid
    #[serde(rename = "${uuid}")]
    Uuid,
    /// Ext
    #[serde(rename = "${ext}")]
    Ext,
}

/// CopyParams is used when copy original files or their modified
/// versions to default storage. Source files MAY either be stored or just
/// uploaded and MUST NOT be deleted
#[derive(Debug, PartialEq, Serialize)]
pub struct CopyParams {
    /// Source is a CDN URL or just ID (UUID) of a file subjected to copy
    pub source: String,
    /// Store parameter only applies to the Uploadcare storage and MUST
    /// be either true or false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<ToStore>,
    /// MakePublic is applicable to custom storage only. MUST be either true or
    /// false. True to make copied files available via public links, false to
    /// reverse the behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub make_public: Option<MakePublic>,
    /// Target identifies a custom storage name related to your project.
    /// Implies you are copying a file to a specified custom storage. Keep in
    /// mind you can have multiple storages associated with a single S3
    /// bucket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Pattern is used to specify file names Uploadcare passes to a custom
    /// storage. In case the parameter is omitted, we use pattern of your
    /// custom storage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Pattern>,
}

/// Holds local_copy response data
#[derive(Debug, Deserialize)]
pub struct LocalCopyInfo {
    /// holds actual data
    pub result: Info,
}

/// Holds remote_copy response data
#[derive(Debug, Deserialize)]
pub struct RemoteCopyInfo {
    /// AlreadyExists is true if destination file with that name
    /// already exists
    #[serde(skip_deserializing)]
    pub already_exists: bool,
    /// Result is a URL with the s3 scheme. Your bucket name is put
    ///  as a host, and an s3 object path follows
    pub result: Option<String>,
}

/// Holds batch operation response data
#[derive(Debug, Deserialize)]
pub struct BatchInfo {
    /// Map of passed files IDs and problems associated problems
    pub problems: Option<HashMap<String, String>>,
    /// Results describes successfully operated files
    pub result: Option<Vec<Info>>,
}
