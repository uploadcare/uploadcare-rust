//! Holds all primitives and logic related file entity.
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

use crate::ucare::{rest::Client, IntoUrlQuery, Result};

/// Service is used to make calls to group API.
pub struct Service<'a> {
    client: &'a Client,
}

/// creates an instance of the group service
pub fn new_svc(client: &Client) -> Service {
    Service { client }
}

impl Service<'_> {
    /// Acquires some file specific info
    pub fn info(&self, group_id: String) -> Result<Info> {
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
    /// }
    /// let mut cur_page = group_svc.list(params)?;
    ///
    /// let mut groups = cur_page.results.unwrap();
    /// while let Some(next_page) = cur_page.next {
    ///     let new_page = group_svc.get_page(next_page)?;
    ///
    ///     groups.extend(new_page.results.unwrap());
    ///
    ///     cur_page = new_page;
    /// }
    ///
    /// for group in groups.iter() {
    ///     println!("group: {}", group);
    /// }
    /// ```
    pub fn list(&self, params: ListParams) -> Result<List> {
        self.client.call::<ListParams, String, List>(
            Method::GET,
            format!("/groups/"),
            Some(params),
            None,
        )
    }

    /// Gets next page by its url
    pub fn get_page(&self, url: String) -> Result<List> {
        let url = Url::parse(url.as_str())?;
        self.client.call_url::<String, List>(Method::GET, url, None)
    }

    /// Marks all files in group as stored
    pub fn store(&self, group_id: String) -> Result<Info> {
        self.client.call::<String, String, Info>(
            Method::PUT,
            format!("/groups/{}/storage/", group_id),
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
    pub datetime_created: Option<String>,
    /// date and time when a group was stored
    pub datetime_stored: Option<String>,
    /// number of files in a group
    pub files_count: i32,
    /// public CDN URL for a group
    pub cdn_url: String,
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
        let mut q = String::new();

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
            q.push_str(Ordering::CreatedAtAsc.to_string().as_str());
        }

        if let Some(val) = self.from {
            q.push('&');
            q.push_str("from=");
            q.push_str(val.as_str());
        }

        q
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
    pub total: Option<f32>,
    /// Number of objects per page.
    pub per_page: Option<f32>,
}
