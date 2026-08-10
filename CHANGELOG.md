## 0.4.0 (Aug 11, 2026)

### REST API v0.7: client core

BREAKING CHANGES:

* **`RestApiVersion::V05` and `V06` are gone**, the client speaks v0.7 only. The enum
  is `#[non_exhaustive]` from now on.
* Error handling reworked: `4xx`/`5xx` responses map to `ErrValue` variants
  (new: `MethodNotAllowed`, `Conflict`, `ServerError`) instead of surfacing serde
  errors; non-JSON and empty error bodies are passed through as text. A missing or
  malformed `Retry-After` header no longer panics.

IMPROVEMENTS:

* Empty success bodies (`204` on the delete endpoints) are handled by the client
  itself; the `"EOF"` substring matching is gone from `webhook::delete` and
  `group::delete`.
* User supplied query values (`from` cursors, add-on `request_id`) are
  percent-encoded; unset list parameters are no longer sent, the documented API
  defaults apply.
* `Warning` response headers (e.g. dropped metadata keys on `local_copy`) are logged.
* The `Date` auth header is formatted with `%Y` instead of ISO week based `%G`,
  which produced invalid signatures around New Year.
* The version in `X-UC-User-Agent` is taken from the crate manifest.

### Files: REST API v0.7

BREAKING CHANGES:

* `file::Service::info` takes an `include: Option<Include>` argument (`appdata`).
* `file::Info`: `datetime_stored`/`datetime_removed` semantics per v0.7; new
  `content_info`, `metadata`, `tags`, `appdata` fields; `size` is `i64`;
  the v0.6-only `source` field is gone. `content_info` types (shared with the
  Upload API) live in `ucare::types` and are re-exported from `file`.
* `ListParams` uses the `Filter` enum for `removed`/`stored` and `Ordering` lost
  sorting by size (not supported by v0.7). `Filter::All` sends no parameter at all:
  `all` is not a documented value.
* `CopyParams`: `make_public` is a plain `Option<bool>` (the documented boolean),
  new `metadata` field (local copy), and `local_copy`/`remote_copy` no longer
  inject implicit `store`/`make_public` defaults — unset fields are not sent.
  `Pattern::AutoFilename` serializes to the documented `${auto_filename}`.
* `VideoStream::frame_rate` is `f64`: NTSC-style fractional rates (`29.97`) are
  common and used to fail deserialization of the whole response.

FEATURES:

* `POST /files/search/` with typed criteria (`SearchQuery`), pagination and
  highlights.
* File tags endpoints: `tags`, `set_tags`, `update_tags`
  (`GET`/`PUT`/`PATCH /files/{uuid}/tags/`).
* File metadata endpoints: `metadata`, `metadata_value`, `set_metadata_value`,
  `delete_metadata_value` (`GET /files/{uuid}/metadata/`,
  `GET`/`PUT`/`DELETE /files/{uuid}/metadata/{key}/`). Keys are validated client
  side against the documented charset before they reach the URL.
* `BatchInfo` exposes the response `status`.

### Conversion: REST API v0.7

BREAKING CHANGES:

* `JobInfo.thumbnails_group_id` renamed to `thumbnails_group_uuid` — the old field
  name never matched the API and always deserialized to `None`.
* `StatusResult.result` is `Option<JobInfo>`: a `failed` job carries no result and
  used to make the whole status call fail to parse.
* `JobParams` has a new `save_in_group` field (document conversion only); `store`
  and `save_in_group` are omitted from the request when unset.

FEATURES:

* `document_info` (`GET /convert/document/{uuid}/`): source format, possible
  conversions and already converted groups. The docs contradict themselves on
  where `converted_groups` lives (top level vs nested in `format`), so both
  placements are accepted; `DocumentInfo::any_converted_groups` picks whichever
  is present.

FIXES:

* `POST /convert/video/` uses the trailing slash — without it the API redirects,
  and a redirected POST loses its body.
* `video_status` uses `GET` and the correct path; conversion job tokens are `i64`.

### Add-Ons: new module (REST API v0.7)

* `addons::Service`: `execute`, `status`, `execute_and_wait`/`wait` for
  `uc_clamav_virus_scan`, `aws_rekognition_detect_labels`,
  `aws_rekognition_detect_moderation_labels` and `remove_bg`, with typed
  per-application params. A transient status poll failure does not lose the
  `request_id` of a running job: it is reported as `Outcome::PollFailed` after
  several consecutive failures.

### Groups: REST API v0.7

BREAKING CHANGES:

* `group::Service::store` is gone: v0.7 removed `PUT /groups/{uuid}/storage/`
  together with the group `datetime_stored` field.
* `group::Info::datetime_created` is a plain `String` (documented as required).

FEATURES:

* `group::Service::delete` (`DELETE /groups/{uuid}/`), new in v0.7.
* `group::Info` carries `files` (with `None` placeholders for removed files) and
  `url` — previously the primary payload of the info endpoint was dropped.

### Project

* `project::Info` exposes the documented `autostore_enabled` field.

### Webhooks: partial update fixes

* `UpdateParams.signing_secret` is only sent when set. It used to be serialized as
  `null` on every update, which contradicted the documented partial-update
  semantics and risked clearing the stored secret.
* `UpdateParams.id` is no longer serialized into the request body (it is a path
  parameter).
* `CreateParams` no longer sends `signing_secret: null`/`is_active: null` for
  unset fields and no longer forces `is_active: true` client side — the API
  default (active) applies.

### Upload API: response schemas

BREAKING CHANGES:

* **`FromUrlData` is now tagged by the response `type` field.** The previous
  `untagged` representation could never produce the `FileInfo` variant — a
  `check_URL_duplicates` hit was silently mis-parsed as a token-less `Token`.
  `FileToken.token` is a plain `String` and the `data_type` field is gone (it
  duplicated the tag).
* `FromUrlStatusData::Progress.total` is `Option<u64>` (documented as nullable) and
  `FromUrlStatusData::Error` carries the documented `error_code`.
* `VideoInfo`/`VideoInfoAudio`/`VideoInfoVideo` numeric fields are integers per the
  documented schema (`frame_rate` stays fractional); **`channels` is `Option<i64>`**
  — it was typed as a string and broke deserialization of any video with sound.
* `GroupInfo.files` is `Option<Vec<Option<FileInfo>>>`: the array holds `null` for
  removed files.

FEATURES:

* `upload::FileInfo` exposes `content_info` and `metadata`, so what is sent on
  upload can also be read back from upload responses.

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
