.PHONY: test test-unit test-integration

test: test-unit test-integration

test-unit:
	cargo test --lib ucare -- --nocapture

test-integration:
	UCARE_SECRET_KEY=$(UCARE_SECRET_KEY) UCARE_PUBLIC_KEY=$(UCARE_PUBLIC_KEY) \
		cargo test --test rest --test upload -- --nocapture
