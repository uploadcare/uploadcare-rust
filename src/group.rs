//! Holds all primitives and logic around the group resource.
//!
//! Individual files on Uploadcare can be joined into groups. Those can be used
//! to better organize your workflow. Technically, groups are ordered lists of
//! files and can hold files together with Image Transformations in their URLs.
//! The most common case with creating groups is when users upload multiple files at once.
//!
//! NOTE: a group itself and files within that group MUST belong to the same project.
//! Groups are immutable and the only way to add/remove a file is creating a new group.
//!
//! Groups are identified in a way similar to individual files.
//! A group ID consists of a UUID followed by a “~” tilde character and a group size:
//! integer number of files in group.
//! For example, here is an identifier for a group holding 12 files:
//!   badfc9f7-f88f-4921-9cc0-22e2c08aa2da~12

use std::fmt::{self, Debug, Display};

use reqwest::{Method, Url};
use serde::Deserialize;

use crate::file;
use crate::ucare::{encode_query_value, rest::Client, IntoUrlQuery, Result};

/// Service is used to make calls to group API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the group service
pub fn new_svc(client: &Client) -> Service<'_> {
    Service { client }
}

impl Service<'_> {
    /// Acquires group specific info, including the list of files in it
    pub fn info(&self, group_id: &str) -> Result<Info> {
        self.client.call::<String, String, Info>(
            Method::GET,
            format!("/groups/{}/", group_id),
            None,
            None,
        )
    }

    /// Returns a list of groups
    ///
    /// ```rust,ignore
    /// # use ucare::group;
    ///
    /// let params = group::ListParams{
    ///     limit: Some(10),
    ///     ordering: Some(group::Ordering::CreatedAtDesc),
    ///     from: None,
    /// };
    /// let list = group_svc.list(params)?;
    /// let mut next_page = list.next;
    ///
    /// let mut groups = list.results.unwrap();
    /// while let Some(next) = next_page {
    ///     let new_page = group_svc.get_page(&next).unwrap();
    ///     next_page = new_page.next;
    ///     groups.extend(new_page.results.unwrap());
    /// }
    ///
    /// for group in groups.iter() {
    ///     println!("group: {:?}", group);
    /// }
    /// ```
    pub fn list(&self, params: ListParams) -> Result<List> {
        self.client.call::<ListParams, String, List>(
            Method::GET,
            "/groups/".to_string(),
            Some(params),
            None,
        )
    }

    /// Gets next page by its url
    pub fn get_page(&self, url: &str) -> Result<List> {
        let url = Url::parse(url)?;
        self.client.call_url::<String, List>(Method::GET, url, None)
    }

    /// Removes a group by its id. Available since APIv0.7 only.
    ///
    /// The files in the group are not affected, only the group itself is
    /// removed.
    pub fn delete(&self, group_id: &str) -> Result<()> {
        self.client.call::<String, String, ()>(
            Method::DELETE,
            format!("/groups/{}/", group_id),
            None,
            None,
        )
    }
}

/// Info holds group specific information
#[derive(Debug, Deserialize)]
pub struct Info {
    /// group identifier
    pub id: String,
    /// date and time when a group was created
    pub datetime_created: String,
    /// number of files in a group
    pub files_count: i32,
    /// public CDN URL for a group
    pub cdn_url: String,
    /// API resource URL for the group
    pub url: Option<String>,
    /// The files in the group, in their original order.
    ///
    /// Only returned by [`Service::info`]; list responses carry no file lists.
    /// An element is `None` when the corresponding file has been removed.
    #[serde(default)]
    pub files: Option<Vec<Option<file::Info>>>,
}

/// Holds all possible params for for the list method
pub struct ListParams {
    /// Specifies preferred amount of groups in a list for a single
    /// response. Defaults to 100, while the maximum is 1000
    pub limit: Option<i32>,
    /// Specifies the way groups are sorted in a returned list.
    /// By default is set to datetime_created.
    pub ordering: Option<Ordering>,
    /// A starting point for filtering group lists. MUST be a datetime value with T used as a
    /// separator. Example: "2015-01-02T10:00:00"
    pub from: Option<String>,
}

/// Specifies the way groups are sorted in a returned list.
/// By default is set to datetime_created.
pub enum Ordering {
    /// datetime_created
    CreatedAtAsc,
    /// -datetime_created
    CreatedAtDesc,
}

impl Display for Ordering {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = match *self {
            Ordering::CreatedAtAsc => "datetime_created",
            Ordering::CreatedAtDesc => "-datetime_created",
        };

        write!(f, "{}", val)
    }
}

impl IntoUrlQuery for ListParams {
    fn into_query(self) -> String {
        // unset parameters are not sent, the documented server defaults apply
        let mut parts: Vec<String> = Vec::new();
        if let Some(val) = self.limit {
            parts.push(format!("limit={}", val));
        }
        if let Some(val) = self.ordering {
            parts.push(format!("ordering={}", val));
        }
        if let Some(ref val) = self.from {
            parts.push(format!("from={}", encode_query_value(val)));
        }

        parts.join("&")
    }
}

/// Holds a list of groups
#[derive(Debug, Deserialize)]
pub struct List {
    /// Actual results
    pub results: Option<Vec<Info>>,
    /// Next page URL.
    pub next: Option<String>,
    /// Previous page URL.
    pub previous: Option<String>,
    /// A total number of objects of the queried type.
    pub total: Option<i32>,
    /// Number of objects per page.
    pub per_page: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_deserializes_without_datetime_stored() {
        // v0.7 dropped datetime_stored together with the ability to mark a group
        // as stored
        let json = r#"{
            "id": "badfc9f7-f88f-4921-9cc0-22e2c08aa2da~12",
            "datetime_created": "2026-08-04T10:00:00Z",
            "files_count": 12,
            "cdn_url": "https://ucarecdn.com/badfc9f7-f88f-4921-9cc0-22e2c08aa2da~12/",
            "url": "https://api.uploadcare.com/groups/badfc9f7-f88f-4921-9cc0-22e2c08aa2da~12/"
        }"#;
        let info: Info = serde_json::from_str(json).unwrap();

        assert_eq!(info.files_count, 12);
        assert_eq!(info.datetime_created, "2026-08-04T10:00:00Z");
        assert_eq!(
            info.url,
            Some("https://api.uploadcare.com/groups/badfc9f7-f88f-4921-9cc0-22e2c08aa2da~12/".to_string()),
        );
        // list responses carry no `files`
        assert!(info.files.is_none());
    }

    #[test]
    fn info_files_may_hold_removed_placeholders() {
        // the files array of the info endpoint contains null for removed files
        let json = r#"{
            "id": "badfc9f7-f88f-4921-9cc0-22e2c08aa2da~2",
            "datetime_created": "2026-08-04T10:00:00Z",
            "files_count": 2,
            "cdn_url": "https://ucarecdn.com/badfc9f7-f88f-4921-9cc0-22e2c08aa2da~2/",
            "files": [
                null,
                {
                    "uuid": "1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6",
                    "size": 12345,
                    "is_image": false,
                    "is_ready": true,
                    "metadata": {}
                }
            ]
        }"#;
        let info: Info = serde_json::from_str(json).unwrap();

        let files = info.files.unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].is_none());
        assert_eq!(
            files[1].as_ref().unwrap().uuid,
            "1f067f79-cbc8-4b61-9c7b-1c1e0ea6b4b6",
        );
    }

    #[test]
    fn list_totals_are_integers() {
        let json = r#"{
            "next": null,
            "previous": null,
            "total": 42,
            "per_page": 100,
            "results": []
        }"#;
        let list: List = serde_json::from_str(json).unwrap();

        assert_eq!(list.total, Some(42));
        assert_eq!(list.per_page, Some(100));
    }

    #[test]
    fn list_params_query_defaults() {
        let params = ListParams {
            limit: None,
            ordering: None,
            from: None,
        };

        // nothing is sent, the documented server side defaults apply
        assert_eq!(params.into_query(), "");
    }

    #[test]
    fn list_params_query_full() {
        let params = ListParams {
            limit: Some(10),
            ordering: Some(Ordering::CreatedAtDesc),
            from: Some("2026-08-04T10:00:00+03:00".to_string()),
        };

        assert_eq!(
            params.into_query(),
            "limit=10&ordering=-datetime_created&from=2026-08-04T10%3A00%3A00%2B03%3A00",
        );
    }
}
