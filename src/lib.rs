#![deny(missing_docs)]

//! The `ucare` crate provides rust client implementation for the Uploadcare REST and upload API.
//!
//! # Usage examples:
//! ```no_run
//! # use std::env;
//!
//! # use env_logger;
//!
//! # use file;
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
//!     let file = ucare::file::new_svc(&rest_client);
//!
//!     // getting a list of files
//!     let list_params = file::ListParams{
//!         limit: Some(10),
//!         ordering: Some(file::Ordering::Size),
//!     }
//!     let list = file.list(params).unwrap();
//!
//!     // getting file info
//!     let file_id = list.results.unwrap()[0].uuid;
//!     let file_info = file.info(file_id).unwrap();
//!
//!     // store file by its id
//!     file.store(file_id).unwrap()
//!
//!     // remove file by its id
//!     file.delete(file_id).unwrap()
//! # }
//! ```
//!
//! # Logging
//! Library uses `log` crate to log useful information.
//! In binary choose a logging implementation and initialize it in the runtime of the program.

mod ucare;

#[cfg(feature = "rest")]
pub use ucare::rest::{ApiVersion as RestApiVersion, Client as RestClient, Config as RestConfig};

#[cfg(feature = "upload")]
pub use ucare::upload::Client as UploadClient;

#[cfg(feature = "rest")]
pub mod conversion;
#[cfg(feature = "rest")]
pub mod file;
#[cfg(feature = "rest")]
pub mod group;

#[cfg(feature = "upload")]
pub mod upload;

pub use ucare::{ApiCreds, ErrValue, Error, Result};
