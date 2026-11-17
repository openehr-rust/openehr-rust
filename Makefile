.PHONY: test-minimum-supported-rust-version

# Check and test on Rust N-2 (the MSRV) and N-1; see bin/make-test-minimum-supported-rust-version.
test-minimum-supported-rust-version:
	bin/make-test-minimum-supported-rust-version
