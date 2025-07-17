# Rust API client for Uploadcare

[![License](https://img.shields.io/github/license/uploadcare/uploadcare-rust)](./LICENSE)
![rest](https://github.com/uploadcare/uploadcare-rust/workflows/test/badge.svg)
[![Documentation](https://docs.rs/uploadcare/badge.svg)](https://docs.rs/uploadcare/)
[![Crates](https://img.shields.io/crates/v/uploadcare.svg)](https://crates.io/crates/uploadcare)

Uploadcare Rust API client that handles uploads and further operations with files by wrapping Uploadcare Upload and REST APIs.

- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Useful links](#useful-links)

## Requirements
 
rustc 1.43   
cargo 1.43

## Installation

```toml
[dependencies]
uploadcare = "^0.1"
```

## Feature Flags

By default the `full` is enabled (REST and Upload API).

To reduce code size, disable default features and enable just the APIs you use:

```toml
# Example: REST API only
uploadcare = { version = "*", default-features = false, features = ["rest"] }
```

## Configuration 

```rust
use ucare;
use ucare::file;
use ucare::upload;

let creds = ucare::apicreds {
    secret_key: "your_project_secret_key",
    pub_key: "your_project_pub_key",
};

// creating rest client
let config = ucare::RestConfig {
    sign_based_auth: true,
    api_version: ucare::RestApiVersion::v06,
};
let rest_client = ucare::RestClient::new(config, creds).unwrap();

// creating upload client
let config = ucare::UploadConfig {
    sign_based_upload: true,
};
let upload_client = ucare::UploadClient::new(config, creds).unwrap();
```

## Usage

For a comprehensive list of examples, check out the [API documentation](https://docs.rs/uploadcare/).
Below are a few usage examples:

```rust
let file_svc = file::new_svc(&rest_client);

let file_id = "b7c1bf20-0f4c-4ba4-b3a8-a74ebc663752";
let file_info = file_svc.info(file_id).unwrap();
println!("{}: {:?}", file_id, file_info);

let upload_svc = upload::new_svc(&upload_client);

let params = upload::FileParams {
    path: "/path/to/file".to_string(),
    name: "filename".to_string(),
    to_store: Some(upload::ToStore::Auto),
};
let file = upload_svc.file(params).unwrap();
println!("uploaded: {:?}", file.id);

```

In examples we’re going to use `ucarecdn.com` domain. Check your project's subdomain in the [Dashboard](https://app.uploadcare.com/projects/-/settings/#delivery).

## Useful links

[Rust API client documentation](https://docs.rs/uploadcare/)  
[Uploadcare documentation](https://uploadcare.com/docs/?utm_source=github&utm_medium=referral&utm_campaign=uploadcare-rust)  
[Upload API reference](https://uploadcare.com/api-refs/upload-api/?utm_source=github&utm_medium=referral&utm_campaign=uploadcare-rust)  
[REST API reference](https://uploadcare.com/api-refs/rest-api/?utm_source=github&utm_medium=referral&utm_campaign=uploadcare-rust)  
[Changelog](https://github.com/uploadcare/uploadcare-rust/blob/master/CHANGELOG.md)  
[Contributing guide](https://github.com/uploadcare/.github/blob/master/CONTRIBUTING.md)  
[Security policy](https://github.com/uploadcare/uploadcare-rust/security/policy)  
[Support](https://github.com/uploadcare/.github/blob/master/SUPPORT.md)  
