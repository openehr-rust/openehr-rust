.PHONY: test-minimum-supported-rust-version

# Check and test on Rust N-2 (the MSRV) and N-1.
test-minimum-supported-rust-version:
	bin/test-minimum-supported-rust-version
