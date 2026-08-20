# Workspace developer gates for the argh (argy) workspace.
#
# clippy -- -D warnings is intentionally not part of `check`: it currently
# fails on pre-existing lints (manual_clear in argh_shared, needless_return in
# argh) that exist on master independent of any feature work. Keep them out of
# the gate until those are resolved so unrelated changes can land cleanly.

test:
    cargo test --workspace --all-features

check:
    cargo fmt --all -- --check
    cargo test --workspace --all-features
