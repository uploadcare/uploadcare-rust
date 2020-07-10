use std::env;

use ucare;

pub fn api_creds() -> ucare::ApiCreds {
    let secret_key = env::var("UCARE_SECRET_KEY").unwrap();
    let pub_key = env::var("UCARE_PUBLIC_KEY").unwrap();

    ucare::ApiCreds {
        secret_key,
        pub_key,
    }
}
