# Uploadcare-Rust

<img 
	align="right"
	width="64"
	height="64"
	src="https://ucarecdn.com/2f4864b7-ed0e-4411-965b-8148623aa680/uploadcare-logo-mark.svg"
	alt=""
/>

![Test](https://github.com/yarikbratashchuk/uploadcare-rust/workflows/Test/badge.svg)
[![Documentation](https://docs.rs/uploadcare/badge.svg)](https://docs.rs/uploadcare/)
[![Crates](https://img.shields.io/crates/v/uploadcare.svg)](https://crates.io/crates/uploadcare)
[![License](https://img.shields.io/github/license/yarikbratashchuk/uploadcare-rust)](./LICENSE)

Rust library for accessing Uploadcare API https://uploadcare.com/

### Cargo.toml

```toml
[dependencies]
uploadcare = "0.1.2"
```

### REST API example 

```rust
use ucare;
use ucare::file;

let creds = ucare::ApiCreds {
    secret_key: "your_project_secret_key",
    pub_key: "your_project_pub_key",
};
let config = ucare::RestConfig {
    sign_based_auth: true,
    api_version: ucare::RestApiVersion::V06,
};
let rest_client = ucare::RestClient::new(config, creds).unwrap();

let file_svc = file::new_svc(&rest_client);

let file_id = "b7c1bf20-0f4c-4ba4-b3a8-a74ebc663752";
let file_info = file_svc.info(file_id).unwrap();
println!("{}: {:?}", file_id, file_info);
```

### Upload API example

```rust
use ucare::upload;

let config = ucare::UploadConfig {
    sign_based_upload: true,
};
let upload_client = ucare::UploadClient::new(config, creds).unwrap();

let upload_svc = upload::new_svc(&upload_client);

let params = upload::FileParams {
    path: "/path/to/file".to_string(),
    name: "filename".to_string(),
    to_store: Some(upload::ToStore::Auto),
};
let file = upload_svc.file(params).unwrap();
println!("uploaded: {:?}", file.id);

```

### Feature Flags

By default the `full` is enabled (REST and Upload API).

To reduce code size, disable default features and enable just the APIs you use:

```toml
# Example: REST API only
uploadcare = { version = "*", default-features = false, features = ["rest"] }
```

----


MIT License
