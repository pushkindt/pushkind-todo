check:
	cargo fmt --all
	cargo clippy --all-features --tests -- -Dwarnings
	cargo test --all-features

coverage:
	cargo tarpaulin --all-features --all-targets --out Html
