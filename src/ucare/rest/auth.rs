//! Authorization related stuff is here

use crypto::{digest::Digest, hmac::Hmac, mac::Mac, md5::Md5, sha1::Sha1};
use itertools::Itertools;
use log::debug;
use reqwest::{blocking::Request, header};

use crate::ucare::ApiCreds;

const AUTH_HEADER_KEY: &str = "Authorization";
const SIMPLE_AUTH_SCHEME: &str = "Uploadcare.Simple";
const SIGN_BASED_AUTH_SCHEME: &str = "Uploadcare";
pub const DATE_HEADER_FORMAT: &str = "%a, %d %h %G %T %Z";

pub fn simple(creds: ApiCreds) -> impl Fn(&mut Request) {
    move |req: &mut Request| {
        let auth = format!(
            "{} {}:{}",
            SIMPLE_AUTH_SCHEME, creds.pub_key, creds.secret_key
        );

        debug!("preparing simple auth param: {}", auth);

        req.headers_mut()
            .insert(AUTH_HEADER_KEY, auth.parse().unwrap());
    }
}

pub fn sign_based(creds: ApiCreds) -> impl Fn(&mut Request) {
    move |req| {
        // getting body hash
        let mut body_data: Vec<u8> = vec![];
        if let Some(data) = req.body() {
            if let Some(bytes) = data.as_bytes() {
                body_data.extend_from_slice(bytes);
            }
        }
        let mut hasher = Md5::new();
        hasher.input(&body_data[..]);
        let body_hash = hasher.result_str();

        // getting path + query
        let parsed_url = req.url();
        let mut path: String = String::from(parsed_url.path());
        if let Some(query) = parsed_url.query() {
            path.push('?');
            path.push_str(query);
        }

        let mut sign_data: String = String::new();
        sign_data.push_str(req.method().as_str());
        sign_data.push('\n');
        sign_data.push_str(&body_hash[..]);
        sign_data.push('\n');
        sign_data.push_str(req.headers()[header::CONTENT_TYPE].to_str().unwrap());
        sign_data.push('\n');
        sign_data.push_str(req.headers()[header::DATE].to_str().unwrap());
        sign_data.push('\n');
        sign_data.push_str(path.as_str());

        let mut mac = Hmac::new(Sha1::new(), creds.secret_key.as_bytes());
        mac.input(sign_data.as_bytes());
        let mac_res = mac.result();
        let signature = mac_res
            .code()
            .iter()
            .format_with("", |byte, f| f(&format_args!("{:02x}", byte)))
            .to_string();

        let auth = format!("{} {}:{}", SIGN_BASED_AUTH_SCHEME, creds.pub_key, signature,);

        debug!("preparing sign based auth param: {}", auth);

        req.headers_mut()
            .insert(AUTH_HEADER_KEY, auth.parse().unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use reqwest::{blocking::Request, Method, Url};

    fn setup_req() -> Request {
        Request::new(
            Method::GET,
            Url::parse("http://testurl.com/files/?limit=1&stored=true").unwrap(),
        )
    }

    #[test]
    fn test_simple() {
        let mut req = setup_req();
        let creds = ApiCreds {
            secret_key: String::from("testsk"),
            pub_key: String::from("testpk"),
        };

        simple(creds)(&mut req);

        assert_eq!(
            req.headers()[AUTH_HEADER_KEY],
            "Uploadcare.Simple testpk:testsk",
        );
    }

    #[test]
    fn test_sign_based() {
        // values are taken from https://uploadcare.com/docs/api_reference/rest/requests_auth/

        let creds = ApiCreds {
            secret_key: "demoprivatekey".to_string(),
            pub_key: "testpk".to_string(),
        };

        let mut req = setup_req();
        let headers = req.headers_mut();

        let now = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1541423681, 0), Utc)
            .format(DATE_HEADER_FORMAT)
            .to_string()
            .replace("UTC", "GMT");

        headers.insert("Date", now.parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        sign_based(creds)(&mut req);

        assert_eq!(
            req.headers()[AUTH_HEADER_KEY],
            "Uploadcare testpk:3cbc4d2cf91f80c1ba162b926f8a975e8bec7995",
        );
    }
}
