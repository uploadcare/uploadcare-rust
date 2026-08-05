//! Types shared between the REST and the Upload API.
//!
//! The two APIs are versioned independently, so most of the response shapes
//! live in their own modules. What ends up here is only what both of them
//! return in exactly the same form.

use serde::Deserialize;

/// Recognized information about the file content.
///
/// REST APIv0.7 returns it as `content_info` of the file object, the Upload API —
/// as `content_info` of its file info responses; the shape is the same.
///
/// All three of the fields are optional: a non media file has neither `image` nor
/// `video`, and files uploaded before the field was introduced may have no `mime`.
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
/// Note the difference from the APIv0.6 `video_info` (`upload::VideoInfo`),
/// which uses the old shape: `video` and `audio` are lists of streams here,
/// `duration` and `bitrate` are nullable, and audio channels are a number
/// rather than a string.
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
#[derive(Debug, PartialEq, Deserialize)]
pub struct VideoStream {
    /// Video stream image height.
    pub height: Option<i64>,
    /// Video stream image width.
    pub width: Option<i64>,
    /// Video stream frame rate.
    ///
    /// A double per the documented schema: fractional NTSC style rates
    /// (`29.97`) are common, do not assume a whole number.
    pub frame_rate: Option<f64>,
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

/// ImageInfo holds image-specific information.
///
/// REST APIv0.7 returns it as `content_info.image` (see [`ContentInfo`]), the
/// Upload API — as `content_info.image` and the legacy `image_info` of
/// `upload::FileInfo`. The set of fields is the same everywhere.
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
