//! Itegration tests for all of the REST API.
//! Very primitive approach.

use rand::Rng;

use ucare::{self, conversion, file, group, project, webhook};

mod testenv;

fn rest_client() -> ucare::RestClient {
    let config = ucare::RestConfig {
        sign_based_auth: true,
        api_version: ucare::RestApiVersion::V07,
    };

    ucare::RestClient::new(config, testenv::api_creds()).unwrap()
}

#[test]
fn file() {
    let client = rest_client();
    let file_svc = file::new_svc(&client);

    let limit = 13;

    let params = file::ListParams {
        removed: Some(file::Filter::False),
        stored: Some(file::Filter::All),
        limit: Some(3),
        ordering: Some(file::Ordering::DatetimeUploaded),
        from: None,
        include: None,
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
    let file = file_svc.info(&first_file.uuid, None).unwrap();

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

    // file copy: POST /files/ is 405 since v0.7, local_copy is the replacement
    let params = file::CopyParams {
        source: file.uuid.to_string(),
        store: None,
        make_public: Some(file::MakePublic::True),
        target: None,
        pattern: None,
    };
    let copy_info = file_svc.local_copy(params).unwrap();

    assert_eq!(copy_info.result.original_filename, file.original_filename);

    // file delete
    let deleted = file_svc.delete(&file.uuid).unwrap();

    assert_ne!(deleted.datetime_removed, None);
}

#[test]
fn search() {
    let client = rest_client();
    let file_svc = file::new_svc(&client);

    // taking any existing file to look for it by an exact uuid match: that path
    // bypasses the search index, so there is no lag to wait for
    let params = file::ListParams {
        removed: Some(file::Filter::False),
        stored: Some(file::Filter::All),
        limit: Some(1),
        ordering: Some(file::Ordering::DatetimeUploaded),
        from: None,
        include: None,
    };
    let existing = file_svc
        .list(params)
        .unwrap()
        .results
        .unwrap()
        .pop()
        .unwrap();

    let params = file::SearchParams {
        query: file::SearchQuery {
            exact: Some(file::Exact {
                uuid: Some(vec![existing.uuid.to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        },
        limit: Some(1),
        offset: None,
        include: None,
    };
    let found = file_svc.search(params).unwrap();

    let results = found.results.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].info.uuid, existing.uuid);

    // no criteria at all is a 400
    let params = file::SearchParams::default();
    assert!(file_svc.search(params).is_err());
}

#[test]
fn group() {
    let client = rest_client();
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
    let client = rest_client();
    let file_svc = file::new_svc(&client);
    let conv_svc = conversion::new_svc(&client);

    let params = file::ListParams {
        removed: Some(file::Filter::False),
        stored: Some(file::Filter::All),
        limit: Some(1),
        ordering: Some(file::Ordering::DatetimeUploaded),
        from: None,
        include: None,
    };
    let list = file_svc.list(params).unwrap();

    // convert file
    let params = conversion::JobParams {
        paths: vec![list.results.unwrap().pop().unwrap().uuid + "/document/-/format/pdf/"],
        store: Some(conversion::ToStore::False),
    };
    let job_result = conv_svc.document(params).unwrap();
    if let Some(mut jobs) = job_result.result {
        if let Some(job) = jobs.pop() {
            let token = job.token.unwrap();

            let status = conv_svc.document_status(token).unwrap();

            assert_eq!(status.error, None);
        }
    }
}

#[test]
fn webhook() {
    let sign_secret = "test_signing_secret";
    let new_sign_secret = "new_signing_secret";

    let client = rest_client();
    let webhook_svc = webhook::new_svc(&client);

    // list
    let list = webhook_svc.list().unwrap();
    assert!(list.len() > 0);
    assert_ne!(list.get(0).unwrap().id, 0);

    // create
    //
    // the host has to resolve to a non private address, so localhost is not an
    // option here: v0.7 rejects it at request validation time
    let mut rng = rand::thread_rng();
    let suff: u8 = rng.gen();
    let target_url = format!("https://example.com/test_endpoint{}", suff);
    let create_params = webhook::CreateParams {
        event: webhook::Event::FileInfoUpdated,
        target_url: target_url.clone(),
        signing_secret: Some(sign_secret.to_string()),
        is_active: None,
        version: None,
    };
    let hook = webhook_svc.create(create_params).unwrap();
    assert!(hook.is_active);
    assert!(hook.created.len() > 0);
    assert!(hook.updated.len() > 0);
    assert_eq!(hook.signing_secret, Some(sign_secret.to_string()));
    // created without an explicit version, still has to end up on 0.7
    assert_eq!(hook.version, "0.7");

    // get by id
    let fetched = webhook_svc.get(hook.id).unwrap();
    assert_eq!(fetched.id, hook.id);
    assert_eq!(fetched.target_url, target_url);

    // subscribing to the same event and url again is a recognizable 400
    let duplicate = webhook_svc.create(webhook::CreateParams {
        event: webhook::Event::FileInfoUpdated,
        target_url: target_url.clone(),
        signing_secret: None,
        is_active: None,
        version: None,
    });
    match duplicate {
        Err(err) => assert!(err.to_string().contains("already subscribed")),
        Ok(_) => panic!("duplicate subscription was accepted"),
    }

    // update
    let update_params = webhook::UpdateParams {
        id: hook.id,
        event: None,
        target_url: None,
        signing_secret: Some(new_sign_secret.to_string()),
        is_active: Some(false),
    };
    let hook = webhook_svc.update(update_params).unwrap();
    assert!(!hook.is_active);
    assert_eq!(hook.signing_secret, Some(new_sign_secret.to_string()));

    // re-enabling a disabled subscription
    let hook = webhook_svc
        .update(webhook::UpdateParams {
            id: hook.id,
            event: None,
            target_url: None,
            signing_secret: None,
            is_active: Some(true),
        })
        .unwrap();
    assert!(hook.is_active);

    // delete: takes every subscription on that url, with a body on a DELETE request
    let delete_params = webhook::DeleteParams { target_url };
    let res = webhook_svc.delete(delete_params).unwrap();
    assert_eq!(res, ());
}

#[test]
fn project() {
    let client = rest_client();
    let project_svc = project::new_svc(&client);

    // info
    let info = project_svc.info().unwrap();
    assert_ne!(info.name, "".to_string());
    assert_ne!(info.pub_key, "".to_string());
}
