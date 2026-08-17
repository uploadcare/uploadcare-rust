## 0.4.0 (Aug 11, 2026)

IMPROVEMENTS:
* Support [API v0.7](https://uploadcare.com/docs/changelog/2026/6/29/)
* Add new REST capabilities: Add-Ons module, file search, file tags and metadata endpoints.
* Improve robustness: error body handling (incl. empty/non-JSON), Retry-After parsing, warning header logging, query value encoding, and Upload API schema fixes (e.g., from_url response tagging, u64 counters, nullable fields).

BREAKING CHANGES:

* `RestApiVersion::V05` and `V06` are gone, the client works with v0.7 only.
* **Subscriptions are now created with version `0.7` instead of `0.6`.**. Receivers written
  against the `0.6` format have to be updated before upgrading, or they will break on
  the first delivery. Subscriptions created earlier are **not** affected — they keep
  their own version and their own delivery format, forever.
* Schema changes (review old contracts on upgrade)

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
