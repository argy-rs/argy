# Workspace developer gates for the argh (argy) workspace.

test:
    cargo test --workspace --all-features

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery
    cargo test --workspace --all-features
