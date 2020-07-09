//! Authorization related stuff is here

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crypto::{hmac::Hmac, mac::Mac, sha2::Sha256};
use itertools::Itertools;

use crate::ucare::ApiCreds;

static SIGNED_UPLOAD_TTL: u32 = 60;

pub(crate) struct Fields {
    pub(crate) pub_key: String,
    pub(crate) signature: Option<String>,
    pub(crate) expire: Option<u32>,
}

pub(crate) fn simple(creds: ApiCreds) -> impl Fn() -> Fields {
    move || -> Fields {
        Fields {
            pub_key: creds.pub_key.clone(),
            signature: None,
            expire: None,
        }
    }
}

pub(crate) fn sign_based(creds: ApiCreds) -> impl Fn() -> Fields {
    move || -> Fields {
        let exp = SystemTime::now()
            .checked_add(Duration::new(SIGNED_UPLOAD_TTL as u64, 0))
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        Fields {
            pub_key: creds.pub_key.clone(),
            signature: Some(get_signature(creds.secret_key.clone(), exp)),
            expire: Some(exp),
        }
    }
}

fn get_signature(secret_key: String, expire: u32) -> String {
    let mut mac = Hmac::new(Sha256::new(), secret_key.as_bytes());
    mac.input(expire.to_string().as_bytes());
    let mac_res = mac.result();

    mac_res
        .code()
        .iter()
        .format_with("", |byte, f| f(&format_args!("{:02x}", byte)))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_based() {
        let secret_key = "project_secret_key".to_string();
        let now = 1454903856;
        let signature = get_signature(secret_key, now);

        assert_eq!(
            "d39a461d41f607338abffee5f31da4d4e46535651c87346e76906bf75c064d47",
            signature,
        );
    }
}
