# uploadcare-rust

<img 
	align="right"
	width="64"
	height="64"
	src="https://ucarecdn.com/2f4864b7-ed0e-4411-965b-8148623aa680/uploadcare-logo-mark.svg"
	alt=""
/>

Rust library for accessing Uploadcare API https://uploadcare.com/

### Table of Contents

- [Requirements](#requirements)
- [Installation](#installation)
- [Documentation](#documentation)
- [Feature Flags](#Feature Flags)

### Requirements

cargo 1.43
rustc 1.43

### Installation

You can start using it by first adding it to your `Cargo.toml`:

```
[dependencies]
uploadcare = "0.1.0"
```

### Documentation

To get started you need to create a client:
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
```

Getting a list of files:
```rust
let file_svc = file::new_svc(&rest_client);

let file_id = "b7c1bf20-0f4c-4ba4-b3a8-a74ebc663752";
let file_info = file_svc.info(file_id).unwrap();
println!("{}: {:?}", file_id, file_info);
```

### Feature Flags

By default the `full` uploadcare api is enabled (REST and Upload API).

To reduce code size, disable default features and enable just the APIs you use:

```toml
# Example: REST API only
uploadare = { version = "*", default-features = false, features = ["rest"] }
```

----


MIT License. Copyright (c) 2020 Uploadcare
