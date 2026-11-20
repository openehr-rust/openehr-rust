.PHONY: ci-minimum-supported-rust-version

# Check and test on Rust N-2 (the MSRV) and N-1.
ci-minimum-supported-rust-version:
	bin/make-ci-minimum-supported-rust-version

.PHONY: test-benchmarks
test-benchmarks:
	bin/test-benchmarks

.PHONY: test-coverage
test-coverage:
	bin/test-coverage
