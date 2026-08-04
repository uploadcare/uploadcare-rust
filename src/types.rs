//! Types shared between the REST and the Upload API.
//!
//! The two APIs are versioned independently, so most of the response shapes
//! live in their own modules. What ends up here is only what both of them
//! return in exactly the same form.

use serde::Deserialize;

/// ImageInfo holds image-specific information.
///
/// REST APIv0.7 returns it as `content_info.image` (see [`crate::file::ContentInfo`]),
/// the Upload API — as `image_info` (see [`crate::upload::FileInfo`]). The set of
/// fields is the same in both.
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
