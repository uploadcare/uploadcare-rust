## Unreleased

### Upload API: request parameters

BREAKING CHANGES:

* **`to_store` left as `None` no longer sends `0`.** All three upload methods used to
  substitute `ToStore::False` for a missing value, which made every upload temporary
  regardless of the project settings. The field is now omitted from the request and
  the API applies its own default — `auto` for projects registered after
  February 12, 2024 and `0` for the older ones. Code that relied on the implicit
  "temporary unless asked otherwise" has to pass `Some(ToStore::False)` explicitly.
* **Byte counters widened from `u32` to `u64`.** `u32` caps at 4 GiB, which multipart
  upload exists to exceed. Affects `MultipartParams::size`, `FileInfo::size`,
  `FileInfo::total`, `FileInfo::done` and the `done` / `total` of
  `FromUrlStatusData::Progress`.
* `upload::FileParams` has two new fields, `metadata: HashMap<String, String>` and
  `tags: Option<Vec<String>>`. Both are `Default`, so `..Default::default()` covers
  them, but an exhaustive struct literal has to be updated.
* `upload::MultipartParams` has three new fields: `part_size: Option<u64>`, plus the
  same `metadata` and `tags`.
* `upload::FromUrlParams` has a new `metadata` field. This endpoint takes metadata but
  no tags.

FEATURES:

* `POST /base/`, `POST /multipart/start/` and `POST /from_url/` now send file metadata
  as `metadata[key]` form fields. The first two also send tags, as a single comma
  separated `tags` field.
* `POST /multipart/start/` accepts `part_size`. Left to the API default of 5 MiB when
  `None`; worth raising for files over a gigabyte, otherwise the number of presigned
  part urls in the response grows into the thousands. Note that whatever is passed
  here decides how the caller has to slice the file — `upload_part` expects every
  part but the last to be exactly that size.

IMPROVEMENTS:

* Tags are passed through as given rather than normalized locally: the API lowercases,
  trims and deduplicates them, so what comes back may differ from what was sent.
  An empty tag vector sends no field at all, same as `None`.

### Webhooks: REST API v0.7

BREAKING CHANGES:

* **Subscriptions are now created with version `0.7` instead of `0.6`.** This is the
  consequence of raising the `Accept` header, and it changes behaviour for existing
  users even though their code does not change: a `0.7` subscription delivers a
  **different payload format** to `target_url` than a `0.6` one did. Receivers written
  against the `0.6` format have to be updated before upgrading, or they will break on
  the first delivery. Subscriptions created earlier are **not** affected — they keep
  their own version and their own delivery format, forever.
* `CreateParams` has a new required field, `version: Option<Version>`. Leave it `None`
  to get `Version::V07`; it is always sent explicitly so that the created subscription
  does not silently depend on which API version the crate happens to speak. Note that
  creating a `0.6` subscription is no longer possible at all: APIv0.7 answers
  `400 Invalid version`, and this crate only speaks v0.7.
* `Info.signing_secret` changed from `String` to `Option<String>`. The API documents
  the field as nullable, so the old type failed to deserialize a subscription without
  a secret.
* `Info.event` stays a `String` rather than becoming the `Event` enum, deliberately:
  reading existing subscriptions can turn up an event a given release does not know
  about, and that must not break deserialization.
* `Event` and the new `Version` enum are `#[non_exhaustive]`.

FEATURES:

* Four new events, all of which require a `0.7` subscription: `Event::FileInfected`,
  `Event::FileStored`, `Event::FileDeleted`, `Event::FileInfoUpdated`.
* `Info.version` exposes the subscription version. It is fixed at creation and can
  never be changed — passing a version to `update` is rejected by the API, which is
  why `UpdateParams` has no such field. Changing a version means deleting the
  subscription and creating it again.
* `webhook::Service::get(id)` reads a single subscription, `GET /webhooks/{id}/`.

IMPROVEMENTS:

* `Info` no longer derives `Debug`; it implements it manually with `signing_secret`
  masked, so the secret does not reach logs through debug output. Code reading the
  field directly still has to mask it itself.
* Documented the nuances that are easy to get wrong:
  * `delete` removes **every** subscription pointing at the given `target_url`, for
    all events at once. It is not a way to unsubscribe from one event.
  * `target_url` must resolve to a non private address, so local endpoints cannot be
    used for debugging — the integration test now uses a public host for that reason.
  * `403` depends on the particular `target_url`, not only on the project, so it must
    not be cached as a per project "webhooks unavailable" flag.
  * `file.info_updated` is not published when nothing actually changed, so it is not a
    confirmation that a write happened; and it also fires for background processing
    that was never requested by the caller.
  * the delivery payload format is defined by the delivery layer, not by the schemas
    of this API — do not reuse `file::Info` for it. In particular `metadata` may be
    `null` in a delivery, while REST API v0.7 always returns an object.

## 0.3.1 (Apr 16, 2026)

IMPROVEMENTS:

* Change document conversion token type from `i32` to `i64`

## 0.3.0 (Dec 4, 2021)

FEATURES:

* Added support for Webhook's [signing secret](https://uploadcare.com/docs/security/secure-webhooks/).

## 0.2.1 (Sep 1, 2020)

IMPROVEMENTS:

* Update file delete endpoint

## 0.2.0 (August 19, 2020)

FEATURES:

* Webhooks
* Project

## 0.1.4 (Jul 24, 2020)

Transfered ownership to Uploadcare

## 0.1.3 (Jul 12, 2020)

IMPROVEMENTS:

* Make some upload methods more convenient

## 0.1.2 (Jul 10, 2020)

FIXES:

* Typos

## 0.1.1 (Jul 10, 2020)

IMPROVEMENTS:

* Pass references to most file and group api methods

## 0.1.0 (July, 2020)

Basic operations
