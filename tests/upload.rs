use rand::Rng;
use std::fs;
use std::io::Read;

use ucare::{self, upload};

mod testenv;

fn upload_client() -> ucare::UploadClient {
    let config = ucare::UploadConfig {
        sign_based_upload: true,
    };

    ucare::UploadClient::new(config, testenv::api_creds()).unwrap()
}

#[test]
fn file_and_group() {
    let mut rng = rand::thread_rng();
    let suff: u8 = rng.gen();

    let client = upload_client();
    let upload_svc = upload::new_svc(&client);

    let filename =
        "London_is_the_capital_of_great_britain_".to_string() + suff.to_string().as_str();

    let params = upload::FileParams {
        path: "./tests/test_image.jpg".to_string(),
        name: filename.to_string(),
        to_store: Some(upload::ToStore::True),
    };
    let short_file_info = upload_svc.file(params).unwrap();

    assert_ne!(short_file_info[filename.as_str()], "".to_string());

    let file_id = short_file_info[filename.as_str()].as_str();
    let file_info = upload_svc.file_info(file_id.as_ref()).unwrap();

    assert_eq!(file_info.file_id, file_id);

    // group
    let group_info = upload_svc
        .create_group(&[(file_id.to_string() + "/-/resize/x800/").as_str()])
        .unwrap();
    let group_info = upload_svc.group_info(group_info.id.as_str()).unwrap();

    assert_eq!(group_info.files.unwrap().len(), 1);
}

#[test]
fn from_url() {
    let client = upload_client();
    let upload_svc = upload::new_svc(&client);

    let params = upload::FromUrlParams {
        source_url:
            "https://www.london.gov.uk/sites/default/files/renew-tower-hamlets-3814-2x1.jpg?v=87935"
                .to_string(),
        to_store: Some(upload::ToStore::True),
        filename: Some("Great_London".to_string()),
        check_url_duplicates: None,
        save_url_duplicates: None,
    };
    let data = upload_svc.from_url(params).unwrap();
    match data {
        upload::FromUrlData::Token(val) => {
            assert_ne!(val.token, None);

            // check status
            let status_data = upload_svc
                .from_url_status(val.token.unwrap().as_str())
                .unwrap();
            println!("{:?}", status_data);
        }
        upload::FromUrlData::FileInfo(info) => {
            assert_ne!(info.uuid, "".to_string());
        }
    };
}

#[test]
fn multipart() {
    let mut rng = rand::thread_rng();
    let suff: u8 = rng.gen();

    let client = upload_client();
    let upload_svc = upload::new_svc(&client);

    let mut data = get_file_chunks("./tests/test_image_2.jpg").unwrap();

    let params = upload::MultipartParams {
        filename: "Porsche_".to_string() + suff.to_string().as_str(),
        size: 10_905_778,
        content_type: "image/jpeg".to_string(),
        to_store: None,
    };
    let multipart_data = upload_svc.multipart_start(params).unwrap();

    for url in multipart_data.parts.iter() {
        upload_svc
            .upload_part(url.as_str(), data.remove(0))
            .unwrap();
    }

    let file_info = upload_svc
        .multipart_complete(multipart_data.uuid.clone())
        .unwrap();

    assert_eq!(file_info.uuid, multipart_data.uuid);
    assert!(file_info.size > 10_000_000);
}

fn get_file_chunks(path: &str) -> ucare::Result<Vec<Vec<u8>>> {
    let mut file = fs::File::open(path)?;
    let mut list_of_chunks = Vec::new();
    let chunk_size: usize = 5_242_880; // 5MB

    loop {
        let mut chunk = Vec::with_capacity(chunk_size);
        let n = file
            .by_ref()
            .take(chunk_size as u64)
            .read_to_end(&mut chunk)?;
        if n == 0 {
            break;
        }
        list_of_chunks.push(chunk);
        if n < chunk_size {
            break;
        }
    }

    Ok(list_of_chunks)
}
