#![deny(missing_docs)]

//! The `ucare` crate provides rust client implementation for the Uploadcare REST and upload API.
//!
//! # Usage examples:
//! ```no_run
//! # use std::env;
//!
//! # use env_logger;
//!
//! # use ucare::file;
//!
//! # fn main() {
//!     let env = env_logger::Env::default()
//!         .filter_or("MY_LOG_LEVEL", "debug")
//!         .write_style_or("MY_LOG_STYLE", "always");
//!
//!     env_logger::init_from_env(env);
//!
//!     let secret_key = env::var("UCARE_SECRET_KEY").unwrap();
//!     let pub_key = env::var("UCARE_PUBLIC_KEY").unwrap();
//!
//!     let creds = ucare::ApiCreds {
//!         secret_key,
//!         pub_key,
//!     };
//!     let config = ucare::RestConfig {
//!         sign_based_auth: true,
//!         api_version: ucare::RestApiVersion::V05,
//!     };
//!
//!     let rest_client = ucare::RestClient::new(config, creds).unwrap();
//!
//!     let file_svc = file::new_svc(&rest_client);
//!
//!     // getting a list of files
//!     let list_params = file::ListParams{
//!         removed: Some(true),
//!         stored: Some(true),
//!         limit: Some(10),
//!         ordering: Some(file::Ordering::Size),
//!         from: None,
//!     };
//!     let list = file_svc.list(list_params).unwrap();
//!
//!     // getting file info
//!     let file_id = &list.results.unwrap()[0].uuid;
//!     let file_info = file_svc.info(file_id.to_string()).unwrap();
//!
//!     // store file by its id
//!     file_svc.store(file_id.to_string()).unwrap();
//!
//!     // remove file by its id
//!     file_svc.delete(file_id.to_string()).unwrap();
//! # }
//! ```
//!
//! # Logging
//! Library uses `log` crate to log useful information.
//! In binary choose a logging implementation and initialize it in the runtime of the program.

mod ucare;

#[cfg(feature = "rest")]
pub use crate::ucare::rest::{
    ApiVersion as RestApiVersion, Client as RestClient, Config as RestConfig,
};

#[cfg(feature = "upload")]
pub use crate::ucare::upload::{Client as UploadClient, Config as UploadConfig};

#[cfg(feature = "rest")]
pub mod conversion;
#[cfg(feature = "rest")]
pub mod file;
#[cfg(feature = "rest")]
pub mod group;

#[cfg(feature = "upload")]
pub mod upload;

pub use crate::ucare::{ApiCreds, ErrValue, Error, Result};
