//! Itegration tests for all of the REST API.
//! Very primitive approach.

use ucare::{self, conversion, file, group};

mod testenv;

fn rest_client_version(version: ucare::RestApiVersion) -> ucare::RestClient {
    let config = ucare::RestConfig {
        sign_based_auth: true,
        api_version: version,
    };

    ucare::RestClient::new(config, testenv::api_creds()).unwrap()
}

fn rest_client_v05() -> ucare::RestClient {
    rest_client_version(ucare::RestApiVersion::V05)
}

fn rest_client_v06() -> ucare::RestClient {
    rest_client_version(ucare::RestApiVersion::V06)
}

#[test]
fn file() {
    let client = rest_client_v05();
    let file_svc = file::new_svc(&client);

    let limit = 13;

    let params = file::ListParams {
        removed: Some(false),
        stored: Some(false),
        limit: Some(3),
        ordering: Some(file::Ordering::Size),
        from: None,
    };

    // file list
    let list = file_svc.list(params).unwrap();
    let mut next_page = list.next;

    let mut files = list.results.unwrap();
    while let Some(next) = next_page {
        let new_page = file_svc.get_page(&next).unwrap();

        next_page = new_page.next;

        files.extend(new_page.results.unwrap());

        if files.len() >= limit as usize {
            break;
        }
    }

    assert!(files.len() >= limit as usize);

    // file info
    let first_file = files.pop().unwrap();
    let file = file_svc.info(&first_file.uuid).unwrap();

    assert_eq!(file.uuid, first_file.uuid);

    // file store
    let info = file_svc.store(&file.uuid).unwrap();

    assert_ne!(info.datetime_stored, None);

    // batch store
    let batch_info = file_svc.batch_store(&[&files.pop().unwrap().uuid]).unwrap();

    assert_ne!(
        batch_info.result.unwrap().pop().unwrap().datetime_stored,
        None
    );

    // file copy
    let params = file::CopyParams {
        source: file.uuid.to_string(),
        store: None,
        make_public: Some(file::MakePublic::True),
        target: None,
        pattern: None,
    };
    let copy_info = file_svc.copy(params).unwrap();

    assert_eq!(copy_info.result.original_filename, file.original_filename);

    // file delete
    let deleted = file_svc.delete(&file.uuid).unwrap();

    assert_ne!(deleted.datetime_removed, None);
}

#[test]
fn group() {
    let client = rest_client_v06();
    let group_svc = group::new_svc(&client);

    let limit = 3;

    // group list
    let params = group::ListParams {
        limit: Some(1),
        ordering: Some(group::Ordering::CreatedAtDesc),
        from: None,
    };
    let list = group_svc.list(params).unwrap();
    let mut next_page = list.next;

    let mut groups = list.results.unwrap();
    while let Some(next) = next_page {
        let new_page = group_svc.get_page(&next).unwrap();
        next_page = new_page.next;
        groups.extend(new_page.results.unwrap());

        if groups.len() >= limit as usize {
            break;
        }
    }

    assert!(groups.len() >= limit as usize);

    // group info
    let first_group = groups.pop().unwrap();
    let group = group_svc.info(&first_group.id).unwrap();

    assert_eq!(group.id, first_group.id);
}

#[test]
fn conversion() {
    let client = rest_client_v06();
    let file_svc = file::new_svc(&client);
    let conv_svc = conversion::new_svc(&client);

    let params = file::ListParams {
        removed: Some(false),
        stored: Some(false),
        limit: Some(1),
        ordering: Some(file::Ordering::Size),
        from: None,
    };
    let list = file_svc.list(params).unwrap();

    // convert file
    let params = conversion::JobParams {
        paths: vec![list.results.unwrap().pop().unwrap().uuid + "/document/-/format/pdf/"],
        store: Some(conversion::ToStore::False),
    };
    let job_result = conv_svc.document(params).unwrap();
    let job = job_result.result.unwrap().pop().unwrap();

    let token = job.token.unwrap();

    let status = conv_svc.document_status(token).unwrap();

    assert_eq!(status.error, None);
}
