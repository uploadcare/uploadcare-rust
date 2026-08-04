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
use serde::{self, ser::SerializeMap, Deserialize, Serialize, Serializer};
use serde_json;

use crate::types::ImageInfo;
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
    ///
    /// Pass `Some(Include::Appdata)` to get the `appdata` field populated, `None`
    /// for a regular request.
    pub fn info(&self, file_id: &str, include: Option<Include>) -> Result<Info> {
        let query = include.map(|val| format!("include={}", val));

        self.client.call::<String, String, Info>(
            Method::GET,
            format!("/files/{}/", file_id),
            query,
            None,
        )
    }

    /// Returns a list of files
    ///
    /// ```rust,ignore
    /// # use ucare::file;
    ///
    /// let params = file::ListParams{
    ///     removed: Some(file::Filter::False),
    ///     stored: Some(file::Filter::All),
    ///     limit: Some(10),
    ///     ordering: Some(file::Ordering::DatetimeUploaded),
    ///     from: None,
    ///     include: None,
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

    /// Searches for the project files. Available since APIv0.7 only.
    ///
    /// At least one of the [`SearchQuery`] criteria has to be set, otherwise the API
    /// answers `400`. The same goes for the rest of the constraints documented on
    /// [`SearchQuery`] and [`SearchParams`] — they are checked server side and
    /// reported back as `ErrValue::BadRequest`, the library does not duplicate the
    /// validation to avoid being stricter than the service.
    ///
    /// ```rust,ignore
    /// # use ucare::file;
    ///
    /// let params = file::SearchParams {
    ///     query: file::SearchQuery {
    ///         query: Some("invoice".to_string()),
    ///         is_image: Some(file::IsImage::False),
    ///         ..Default::default()
    ///     },
    ///     limit: Some(50),
    ///     offset: None,
    ///     include: None,
    /// };
    /// let found = file_svc.search(params)?;
    ///
    /// for f in found.results.unwrap().iter() {
    ///     println!("{}: {:?}", f.info.uuid, f.highlight.original_filename);
    /// }
    /// ```
    ///
    /// Note that the search index lags behind the actual state by tens of seconds:
    /// a freshly uploaded file may not be found yet, and a freshly deleted one may
    /// still be listed.
    pub fn search(&self, params: SearchParams) -> Result<SearchList> {
        let query = params.pagination_query();
        let json = encode_json(&params.query)?;

        self.client.call::<String, Vec<u8>, SearchList>(
            Method::POST,
            "/files/search/".to_string(),
            query,
            Some(json),
        )
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
            format!("/files/{}/storage/", file_id),
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
    ///
    /// Also set for a file that is not in the storage yet but was marked to be stored
    /// on upload, in which case it holds the upload time.
    pub datetime_stored: Option<String>,
    /// Date and time when a file was uploaded.
    pub datetime_uploaded: Option<String>,
    /// Is file is image.
    ///
    /// Three-state: `Some(true)` is a recognized image, `Some(false)` is definitely
    /// not an image and `None` means recognition has not finished yet.
    pub is_image: Option<bool>,
    /// Is file is ready to be used after upload.
    pub is_ready: Option<bool>,
    /// File MIME-type as declared on upload.
    ///
    /// The detected one is in `content_info.mime`.
    pub mime_type: Option<String>,
    /// Publicly available file CDN URL. Available if a file is not deleted.
    pub original_file_url: Option<String>,
    /// Original file name taken from uploaded file.
    pub original_filename: Option<String>,
    /// File size in bytes.
    pub size: Option<i64>,
    /// API resource URL for a particular file.
    pub url: Option<String>,
    /// Dictionary of other files that has been created using this file as source. Used for video,
    /// document and etc. conversion.
    pub variations: Option<serde_json::Value>,
    /// File upload source. This field contains information about from where file was uploaded, for
    /// example: facebook, gdrive, gphotos, etc.
    pub source: Option<String>,
    /// Recognized content information: mime type, image and video metadata.
    ///
    /// Replaces `image_info` and `video_info` of APIv0.6. Is `None` for files whose
    /// content was never recognized or the recognition failed.
    pub content_info: Option<ContentInfo>,
    /// Arbitrary user defined `key -> value` pairs attached to the file.
    ///
    /// The API always returns an object here, an empty one when there is no metadata,
    /// hence not an `Option`.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// File tags.
    ///
    /// Three states to tell apart: `None` means the feature is disabled for the
    /// project, `Some([])` means the file has no tags, and a non empty vector holds
    /// the tags themselves. The order is significant and must not be changed: it is
    /// the order of the first occurrence, not a sorted set.
    pub tags: Option<Vec<String>>,
    /// Results produced by applications (virus scan, object recognition and so on),
    /// keyed by the application id.
    ///
    /// Only present when `appdata` was asked for through
    /// [`ListParams::include`], otherwise `None`.
    pub appdata: Option<HashMap<String, AppDataEntry>>,
}

/// Recognized information about the file content.
///
/// All three of the fields are optional: a non media file has neither `image` nor
/// `video`, and files uploaded before the field was introduced may have no `mime`
/// (the MIME type declared on upload is always available as `Info::mime_type`).
#[derive(Debug, Deserialize)]
pub struct ContentInfo {
    /// Detected MIME type.
    pub mime: Option<MimeInfo>,
    /// Image metadata.
    pub image: Option<ImageInfo>,
    /// Video metadata.
    pub video: Option<VideoInfo>,
}

/// Detected MIME type, split into parts
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct MimeInfo {
    /// Full MIME type, `image/jpeg` for example.
    pub mime: Option<String>,
    /// Type part, `image` for example.
    #[serde(rename = "type")]
    pub mime_type: Option<String>,
    /// Subtype part, `jpeg` for example.
    pub subtype: Option<String>,
}

/// Video related information
///
/// Note the difference from the APIv0.6 `video_info` and from
/// [`crate::upload::VideoInfo`], which still uses the old shape: `video` and `audio`
/// are lists of streams here, `duration` and `bitrate` are nullable, and audio
/// channels are a number rather than a string.
#[derive(Debug, PartialEq, Deserialize)]
pub struct VideoInfo {
    /// Video format (MP4 for example).
    pub format: Option<String>,
    /// Video duration in milliseconds.
    pub duration: Option<i64>,
    /// Video bitrate.
    pub bitrate: Option<i64>,
    /// Video streams. Empty for files without a video stream, an audio file for example.
    #[serde(default)]
    pub video: Vec<VideoStream>,
    /// Audio streams. Empty when the file has no sound.
    #[serde(default)]
    pub audio: Vec<AudioStream>,
}

/// A single video stream of a video file
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct VideoStream {
    /// Video stream image height.
    pub height: Option<i64>,
    /// Video stream image width.
    pub width: Option<i64>,
    /// Video stream frame rate, already rounded by the API.
    pub frame_rate: Option<i64>,
    /// Video stream bitrate.
    pub bitrate: Option<i64>,
    /// Video stream codec.
    pub codec: Option<String>,
}

/// A single audio stream of a video file
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct AudioStream {
    /// Audio stream number of channels.
    pub channels: Option<i64>,
    /// Audio stream bitrate.
    pub bitrate: Option<i64>,
    /// Audio stream codec.
    pub codec: Option<String>,
    /// Audio stream sample rate.
    pub sample_rate: Option<i64>,
    /// Audio stream profile.
    pub profile: Option<String>,
}

/// Result produced by a single application for a file
#[derive(Debug, Deserialize)]
pub struct AppDataEntry {
    /// Application output.
    ///
    /// Opaque on purpose: the shape is defined by the application and its `version`,
    /// so it is left as raw json rather than validated.
    #[serde(default)]
    pub data: serde_json::Value,
    /// Version of the application data format.
    pub version: Option<String>,
    /// When the entry was created.
    pub datetime_created: Option<String>,
    /// When the entry was last updated.
    pub datetime_updated: Option<String>,
}

/// Holds all possible params for for the list method
pub struct ListParams {
    /// Set to `Filter::True` to only include removed files in the response,
    /// `Filter::False` to only include existing ones and `Filter::All` to include
    /// both. Defaults to `Filter::False`.
    pub removed: Option<Filter>,
    /// Set to `Filter::True` to only include files that were stored,
    /// `Filter::False` to only include temporary ones and `Filter::All` to include
    /// both. The default is unset, which is the same as `Filter::All`.
    pub stored: Option<Filter>,
    /// Specifies preferred amount of files in a list for a single
    /// response. Defaults to 100, while the maximum is 1000
    pub limit: Option<i32>,
    /// Specifies the way files are sorted in a returned list.
    /// By default is set to datetime_uploaded.
    pub ordering: Option<Ordering>,
    /// Specifies a starting point for filtering files.
    /// The value depends on your ordering parameter value.
    pub from: Option<String>,
    /// Additional fields to be included into every returned file.
    pub include: Option<Include>,
}

/// A three valued filter for the list method.
///
/// `All` was added in APIv0.7, before that the parameters were plain booleans.
/// Note that `removed: All` combined with `stored: All` is a valid request, while
/// `removed: True` combined with `stored: True` returns an empty result — that is
/// expected, not an error.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Filter {
    /// "true"
    True,
    /// "false"
    False,
    /// "all"
    All,
}

impl Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            Filter::True => "true",
            Filter::False => "false",
            Filter::All => "all",
        };

        write!(f, "{}", val)
    }
}

/// Specifies the way files are sorted in a returned list.
/// By default is set to datetime_uploaded.
///
/// Sorting by size was supported by APIv0.6 but is gone in v0.7: it breaks cursor
/// based pagination when a whole page holds files of the same size. Any unsupported
/// value now makes the API answer `400` instead of silently falling back to the
/// default, so keeping this an enum is what keeps such a request from being made.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Ordering {
    /// "datetime_uploaded"
    DatetimeUploaded,
    /// "-datetime_uploaded"
    DatetimeUploadedNeg,
}

impl Display for Ordering {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            Ordering::DatetimeUploaded => "datetime_uploaded",
            Ordering::DatetimeUploadedNeg => "-datetime_uploaded",
        };

        write!(f, "{}", val)
    }
}

/// Additional fields to be included into the response.
///
/// Replaces the `add_fields=rekognition_info` parameter of APIv0.6.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Include {
    /// Include `appdata` into every returned file.
    ///
    /// Notably more expensive than a regular request, since it pulls in the related
    /// application records, so do not enable it by default.
    Appdata,
}

impl Display for Include {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            Include::Appdata => "appdata",
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
            q.push_str(Filter::False.to_string().as_str());
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
            q.push_str("100");
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

        if let Some(val) = self.include {
            q.push('&');
            q.push_str("include=");
            q.push_str(val.to_string().as_str());
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
    /// Number of files in the project broken down by their storage state,
    /// regardless of the query parameters.
    pub totals: Option<Totals>,
    /// Number of objects per page.
    pub per_page: Option<i32>,
}

/// A breakdown of the project files by their storage state
#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Totals {
    /// Number of files marked as removed.
    pub removed: Option<i32>,
    /// Number of files in the storage.
    pub stored: Option<i32>,
    /// Number of uploaded but not stored files.
    pub unstored: Option<i32>,
}

/// Holds all possible params for the search method
///
/// The `query` goes into the request body, the rest of the fields into the query
/// string.
#[derive(Debug, Default)]
pub struct SearchParams {
    /// What to search for.
    pub query: SearchQuery,
    /// Preferred amount of files in a single response, 1 to 100.
    /// Defaults to 20 on the API side.
    pub limit: Option<i32>,
    /// Number of files to skip. Defaults to 0.
    ///
    /// `offset + limit` MUST NOT exceed 1000, otherwise the API answers `400`. This
    /// is a hard cap on the result depth: walking the whole project through the
    /// search is not possible, use [`Service::list`] for that.
    pub offset: Option<i32>,
    /// Additional fields to be included into every found file.
    pub include: Option<Include>,
}

impl SearchParams {
    /// Builds the query string part of the search request, `None` when there is
    /// nothing to put into it.
    fn pagination_query(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if let Some(val) = self.limit {
            parts.push(format!("limit={}", val));
        }
        if let Some(val) = self.offset {
            parts.push(format!("offset={}", val));
        }
        if let Some(ref val) = self.include {
            parts.push(format!("include={}", val));
        }

        if parts.is_empty() {
            return None;
        }
        Some(parts.join("&"))
    }
}

/// Search criteria. At least one of the fields MUST be set.
#[derive(Debug, Default, Serialize)]
pub struct SearchQuery {
    /// Full text search over several fields at once. At least 4 characters long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Substring search in specific fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrase: Option<Phrase>,
    /// Exact match search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact: Option<Exact>,
    /// Upload time range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime_uploaded: Option<DateRange>,
    /// File size range, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<SizeRange>,
    /// Whether the file is a recognized image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_image: Option<IsImage>,
    /// File tags to match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<TagsFilter>,
    /// Allow for typos in the full text search. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuzziness: Option<bool>,
    /// Result ordering, up to 4 entries.
    ///
    /// Duplicates and opposite directions of the same field (`Size` together with
    /// `SizeNeg`) are rejected with a `400`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<Sort>>,
}

/// Substring search in specific fields.
///
/// Every value has to be at least 4 characters long. A field MUST NOT be used in
/// both [`Phrase`] and [`Exact`] within one query, that is a `400`.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Phrase {
    /// Search in the detected MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_mime_type: Option<String>,
    /// Search in the file metadata values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    /// Search in the original file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
}

/// Exact match search. Every set field has to hold a non empty list of values.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Exact {
    /// Match any of the given UUIDs.
    pub uuid: Option<Vec<String>>,
    /// Match any of the given detected MIME types.
    pub detected_mime_type: Option<Vec<String>>,
    /// Match any of the given original file names.
    pub original_filename: Option<Vec<String>>,
    /// Match file metadata: `metadata key -> any of the given values`.
    ///
    /// Serialized as `metadata[key]` entries next to the fields above. Keys are
    /// limited to 64 characters and values to 512, same as the file metadata itself.
    pub metadata: HashMap<String, Vec<String>>,
}

impl Serialize for Exact {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // the `metadata[key]` keys cannot be expressed with a derive, hence the
        // hand written map
        let mut len = self.metadata.len();
        for field in [
            &self.uuid,
            &self.detected_mime_type,
            &self.original_filename,
        ]
        .iter()
        {
            if field.is_some() {
                len += 1;
            }
        }

        let mut map = serializer.serialize_map(Some(len))?;
        if let Some(ref val) = self.uuid {
            map.serialize_entry("uuid", val)?;
        }
        if let Some(ref val) = self.detected_mime_type {
            map.serialize_entry("detected_mime_type", val)?;
        }
        if let Some(ref val) = self.original_filename {
            map.serialize_entry("original_filename", val)?;
        }
        for (key, val) in self.metadata.iter() {
            map.serialize_entry(format!("metadata[{}]", key).as_str(), val)?;
        }
        map.end()
    }
}

/// A date range. At least one of the bounds MUST be set.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct DateRange {
    /// Greater than.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gt: Option<String>,
    /// Greater than or equal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gte: Option<String>,
    /// Less than.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<String>,
    /// Less than or equal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lte: Option<String>,
}

/// A file size range in bytes. At least one of the bounds MUST be set.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct SizeRange {
    /// Greater than.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gt: Option<i64>,
    /// Greater than or equal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gte: Option<i64>,
    /// Less than.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<i64>,
    /// Less than or equal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lte: Option<i64>,
}

/// Tag based search criteria. At least one of the fields MUST be set.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct TagsFilter {
    /// File has at least one of these tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<String>>,
    /// File has all of these tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<String>>,
    /// File has none of these tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none: Option<Vec<String>>,
}

/// Value of the `is_image` search criterion.
///
/// Mirrors the three states of [`Info::is_image`]. Serialized as a real json
/// boolean or `null`: the API rejects the strings `"true"` and `"false"` with
/// a `400`.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum IsImage {
    /// A recognized image.
    True,
    /// Definitely not an image.
    False,
    /// Recognition has not finished yet.
    Unknown,
}

impl Serialize for IsImage {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            IsImage::True => serializer.serialize_bool(true),
            IsImage::False => serializer.serialize_bool(false),
            IsImage::Unknown => serializer.serialize_none(),
        }
    }
}

/// Specifies the way found files are sorted.
///
/// Sorting by size is available here, unlike in [`Ordering`] for the file list:
/// the limitation there comes from cursor based pagination, which the search
/// does not use.
#[non_exhaustive]
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub enum Sort {
    /// "score"
    #[serde(rename = "score")]
    Score,
    /// "-score"
    #[serde(rename = "-score")]
    ScoreNeg,
    /// "size"
    #[serde(rename = "size")]
    Size,
    /// "-size"
    #[serde(rename = "-size")]
    SizeNeg,
    /// "datetime_uploaded"
    #[serde(rename = "datetime_uploaded")]
    DatetimeUploaded,
    /// "-datetime_uploaded"
    #[serde(rename = "-datetime_uploaded")]
    DatetimeUploadedNeg,
    /// "original_filename"
    #[serde(rename = "original_filename")]
    OriginalFilename,
    /// "-original_filename"
    #[serde(rename = "-original_filename")]
    OriginalFilenameNeg,
}

/// Holds the search results
#[derive(Debug, Deserialize)]
pub struct SearchList {
    /// Actual results
    pub results: Option<Vec<SearchResult>>,
    /// Next page URL, `None` when the end of the results is reached.
    pub next: Option<String>,
    /// Previous page URL, `None` when the offset is 0.
    pub previous: Option<String>,
    /// A total number of matched files.
    ///
    /// Approximate: the search index lags behind, and recently removed files are
    /// filtered out of the results with `total` adjusted accordingly. Do not build
    /// invariants like "there are exactly `ceil(total / limit)` pages" on it.
    pub total: Option<i32>,
    /// Number of objects per page.
    pub per_page: Option<i32>,
}

/// A single search result: a file plus the matched fragments
#[derive(Debug, Deserialize)]
pub struct SearchResult {
    /// The file itself.
    #[serde(flatten)]
    pub info: Info,
    /// Fragments of the matched values. Always present, but may be empty.
    #[serde(default)]
    pub highlight: Highlight,
}

/// Fragments of the values a file was matched by
#[derive(Debug, Default, Deserialize)]
pub struct Highlight {
    /// Matched fragments of the original file name.
    #[serde(default)]
    pub original_filename: Vec<String>,
    /// Matched fragments of the detected MIME type.
    #[serde(default)]
    pub detected_mime_type: Vec<String>,
    /// Matched metadata: `metadata key -> value fragment`.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal v0.7 file object, the fields every response is required to carry.
    fn minimal_info() -> &'static str {
        r#"{
            "uuid": "1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6",
            "size": 12345,
            "mime_type": "image/jpeg",
            "is_image": true,
            "is_ready": true,
            "original_filename": "test.jpg",
            "original_file_url": null,
            "url": "https://api.uploadcare.com/files/1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6/",
            "datetime_uploaded": "2026-08-04T10:00:00Z",
            "datetime_stored": null,
            "datetime_removed": null,
            "variations": null,
            "content_info": null,
            "metadata": {}
        }"#
    }

    #[test]
    fn info_deserializes_minimal_response() {
        let info: Info = serde_json::from_str(minimal_info()).unwrap();

        assert_eq!(info.size, Some(12345));
        assert!(info.metadata.is_empty());
        // absent, not empty: the tags feature is off for the project
        assert_eq!(info.tags, None);
        // appdata is only returned when explicitly asked for
        assert!(info.appdata.is_none());
    }

    #[test]
    fn info_size_holds_files_over_2gb() {
        let json = minimal_info().replace("\"size\": 12345", "\"size\": 3221225472");
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(info.size, Some(3_221_225_472));
    }

    #[test]
    fn info_is_image_is_three_state() {
        let json = minimal_info().replace("\"is_image\": true", "\"is_image\": null");
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        // recognition has not finished yet, which is not the same as "not an image"
        assert_eq!(info.is_image, None);
    }

    #[test]
    fn info_metadata_is_parsed() {
        let json = minimal_info().replace(
            "\"metadata\": {}",
            "\"metadata\": {\"subsystem\": \"uploader\", \"pk\": \"17\"}",
        );
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(info.metadata.len(), 2);
        assert_eq!(
            info.metadata.get("subsystem"),
            Some(&"uploader".to_string())
        );
    }

    #[test]
    fn info_tags_tell_empty_from_missing() {
        let json = minimal_info().replace("\"metadata\": {}", "\"metadata\": {}, \"tags\": []");
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(info.tags, Some(vec![]));
    }

    #[test]
    fn content_info_video_streams_are_lists() {
        let json = minimal_info().replace(
            "\"content_info\": null",
            r#""content_info": {
                "mime": {"mime": "video/mp4", "type": "video", "subtype": "mp4"},
                "video": {
                    "format": "MP4",
                    "duration": 10000,
                    "bitrate": 1000,
                    "video": [
                        {"width": 1920, "height": 1080, "frame_rate": 30, "bitrate": 2000, "codec": "h264"}
                    ],
                    "audio": [
                        {"bitrate": 128, "codec": "aac", "sample_rate": 44100, "channels": 2, "profile": null}
                    ]
                }
            }"#,
        );
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        let content_info = info.content_info.unwrap();
        assert_eq!(
            content_info.mime.unwrap().mime_type,
            Some("video".to_string())
        );

        let video = content_info.video.unwrap();
        assert_eq!(video.duration, Some(10000));
        assert_eq!(video.video.len(), 1);
        // integer in v0.7, was a float in the v0.6 video_info
        assert_eq!(video.video[0].frame_rate, Some(30));
        // a number in v0.7, was a string in the v0.6 video_info
        assert_eq!(video.audio[0].channels, Some(2));
    }

    #[test]
    fn content_info_video_streams_may_be_empty() {
        // an audio file: no video streams at all
        let json = minimal_info().replace(
            "\"content_info\": null",
            r#""content_info": {
                "video": {
                    "format": "MP3",
                    "duration": null,
                    "bitrate": null,
                    "video": [],
                    "audio": [{"bitrate": 128, "codec": "mp3", "sample_rate": 44100, "channels": 2}]
                }
            }"#,
        );
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        let video = info.content_info.unwrap().video.unwrap();
        assert!(video.video.is_empty());
        assert_eq!(video.duration, None);
        assert_eq!(video.bitrate, None);
    }

    #[test]
    fn content_info_ignores_unknown_keys() {
        // the content detector keeps growing, unknown keys must not break parsing
        let json = minimal_info().replace(
            "\"content_info\": null",
            r#""content_info": {"something_new": {"a": 1}}"#,
        );
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        let content_info = info.content_info.unwrap();
        assert!(content_info.mime.is_none());
        assert!(content_info.image.is_none());
        assert!(content_info.video.is_none());
    }

    #[test]
    fn appdata_keeps_application_output_opaque() {
        let json = minimal_info().replace(
            "\"metadata\": {}",
            r#""metadata": {}, "appdata": {
                "uc_clamav_virus_scan": {
                    "data": {"infected": false},
                    "version": "0.104.2",
                    "datetime_created": "2026-08-04T10:00:00Z",
                    "datetime_updated": "2026-08-04T10:00:00Z"
                },
                "remove_bg": {
                    "data": {"foreground_type": "person"},
                    "version": null,
                    "datetime_created": "2026-08-04T10:00:00Z",
                    "datetime_updated": "2026-08-04T10:00:00Z"
                }
            }"#,
        );
        let info: Info = serde_json::from_str(json.as_str()).unwrap();

        let appdata = info.appdata.unwrap();
        assert_eq!(appdata.len(), 2);

        let scan = appdata.get("uc_clamav_virus_scan").unwrap();
        assert_eq!(scan.version, Some("0.104.2".to_string()));
        assert_eq!(scan.data["infected"], serde_json::json!(false));
        // version is nullable for applications that never recorded one
        assert_eq!(appdata.get("remove_bg").unwrap().version, None);
    }

    #[test]
    fn list_params_query_defaults() {
        let params = ListParams {
            removed: None,
            stored: None,
            limit: None,
            ordering: None,
            from: None,
            include: None,
        };

        assert_eq!(
            params.into_query(),
            "removed=false&limit=100&ordering=datetime_uploaded",
        );
    }

    #[test]
    fn list_params_query_full() {
        let params = ListParams {
            removed: Some(Filter::True),
            stored: Some(Filter::True),
            limit: Some(10),
            ordering: Some(Ordering::DatetimeUploadedNeg),
            from: Some("2026-08-04T10:00:00Z".to_string()),
            include: Some(Include::Appdata),
        };

        assert_eq!(
            params.into_query(),
            "removed=true&stored=true&limit=10&ordering=-datetime_uploaded\
             &from=2026-08-04T10:00:00Z&include=appdata",
        );
    }

    #[test]
    fn list_params_query_all_filter() {
        let params = ListParams {
            removed: Some(Filter::All),
            stored: Some(Filter::All),
            limit: None,
            ordering: None,
            from: None,
            include: None,
        };

        assert_eq!(
            params.into_query(),
            "removed=all&stored=all&limit=100&ordering=datetime_uploaded",
        );
    }

    #[test]
    fn search_query_serializes_only_what_is_set() {
        let query = SearchQuery {
            query: Some("invoice".to_string()),
            sort: Some(vec![Sort::ScoreNeg, Sort::SizeNeg]),
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(&query).unwrap(),
            serde_json::json!({"query": "invoice", "sort": ["-score", "-size"]}),
        );
    }

    #[test]
    fn search_query_is_image_serializes_as_json_boolean() {
        // strings "true"/"false" are rejected by the API with a 400
        let as_value = |val: IsImage| {
            serde_json::to_value(SearchQuery {
                is_image: Some(val),
                ..Default::default()
            })
            .unwrap()
        };

        assert_eq!(
            as_value(IsImage::True),
            serde_json::json!({"is_image": true})
        );
        assert_eq!(
            as_value(IsImage::False),
            serde_json::json!({"is_image": false}),
        );
        assert_eq!(
            as_value(IsImage::Unknown),
            serde_json::json!({"is_image": null}),
        );
    }

    #[test]
    fn search_query_exact_serializes_metadata_keys() {
        let mut metadata = HashMap::new();
        metadata.insert("subsystem".to_string(), vec!["uploader".to_string()]);

        let query = SearchQuery {
            exact: Some(Exact {
                uuid: Some(vec!["1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6".to_string()]),
                metadata,
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(&query).unwrap(),
            serde_json::json!({"exact": {
                "uuid": ["1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6"],
                "metadata[subsystem]": ["uploader"],
            }}),
        );
    }

    #[test]
    fn search_query_ranges_are_serialized() {
        let query = SearchQuery {
            size: Some(SizeRange {
                gte: Some(1024),
                lt: Some(3_221_225_472),
                ..Default::default()
            }),
            datetime_uploaded: Some(DateRange {
                gt: Some("2026-08-01T00:00:00Z".to_string()),
                ..Default::default()
            }),
            tags: Some(TagsFilter {
                any: Some(vec!["invoice".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(&query).unwrap(),
            serde_json::json!({
                "size": {"gte": 1024, "lt": 3221225472i64},
                "datetime_uploaded": {"gt": "2026-08-01T00:00:00Z"},
                "tags": {"any": ["invoice"]},
            }),
        );
    }

    #[test]
    fn search_query_without_criteria_serializes_empty() {
        // the API answers 400 for this, the library passes it through rather than
        // duplicating the validation
        assert_eq!(
            serde_json::to_value(SearchQuery::default()).unwrap(),
            serde_json::json!({}),
        );
    }

    #[test]
    fn search_params_pagination_query() {
        let params = SearchParams {
            query: SearchQuery::default(),
            limit: Some(50),
            offset: Some(100),
            include: Some(Include::Appdata),
        };

        assert_eq!(
            params.pagination_query(),
            Some("limit=50&offset=100&include=appdata".to_string()),
        );

        // nothing to put into the query string, the API defaults apply
        assert_eq!(SearchParams::default().pagination_query(), None);
    }

    #[test]
    fn search_result_holds_file_and_highlight() {
        let json = format!(
            r#"{{"next": null, "previous": null, "total": 1, "per_page": 20,
                 "results": [{{"highlight": {{
                     "original_filename": ["<em>invoice</em>.pdf"],
                     "metadata": {{"subsystem": "<em>uploader</em>"}}
                 }}, {}}}]}}"#,
            // the file itself is flattened into the same object
            minimal_info().trim_start_matches('{').trim_end_matches('}'),
        );
        let found: SearchList = serde_json::from_str(json.as_str()).unwrap();

        let results = found.results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].info.uuid, "1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6",);
        assert_eq!(
            results[0].highlight.original_filename,
            vec!["<em>invoice</em>.pdf".to_string()],
        );
        assert_eq!(
            results[0].highlight.metadata.get("subsystem"),
            Some(&"<em>uploader</em>".to_string()),
        );
        // not matched by this field
        assert!(results[0].highlight.detected_mime_type.is_empty());
    }

    #[test]
    fn search_result_accepts_empty_highlight() {
        let json = format!(
            r#"{{"results": [{{"highlight": {{}}, {}}}]}}"#,
            minimal_info().trim_start_matches('{').trim_end_matches('}'),
        );
        let found: SearchList = serde_json::from_str(json.as_str()).unwrap();

        let results = found.results.unwrap();
        assert!(results[0].highlight.original_filename.is_empty());
        assert!(results[0].highlight.metadata.is_empty());
    }

    #[test]
    fn list_deserializes_totals() {
        let json = r#"{
            "next": null,
            "previous": null,
            "total": 3,
            "totals": {"removed": 1, "stored": 2, "unstored": 0},
            "per_page": 100,
            "results": []
        }"#;
        let list: List = serde_json::from_str(json).unwrap();

        assert_eq!(list.total, Some(3));
        assert_eq!(
            list.totals,
            Some(Totals {
                removed: Some(1),
                stored: Some(2),
                unstored: Some(0),
            }),
        );
    }
}
