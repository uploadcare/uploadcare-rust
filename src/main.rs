use std::env;
use ucare;
use ucare::file;

fn main() {
    let secret_key = env::var("UCARE_SECRET_KEY").unwrap();
    let pub_key = env::var("UCARE_PUBLIC_KEY").unwrap();

    println!("{} {}", secret_key, pub_key);

    let creds = ucare::ApiCreds {
        secret_key,
        pub_key,
    };
    let config = ucare::RestConfig {
        sign_based_auth: true,
        api_version: ucare::RestApiVersion::V05,
    };

    let client = ucare::RestClient::new(config, creds).unwrap();

    let file = file::new_svc(&client);

    let file_id = "b7c1bf20-0f4c-4ba4-b3a8-a74ebc663752";

    // getting file info
    let file_info = file.info(file_id).unwrap();
    println!("{}: {:?}", file_id, file_info);
}
