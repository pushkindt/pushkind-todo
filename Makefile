.PHONY: check coverage run

run:
	cd frontend && npm run build
	cargo run

check:
	cargo fmt --all
	cargo clippy --all-features --tests -- -Dwarnings
	cargo test --all-features
	cd frontend && npm run format
	cd frontend && npm run lint
	cd frontend && npm test

coverage:
	cargo tarpaulin --all-features --all-targets --out Html
