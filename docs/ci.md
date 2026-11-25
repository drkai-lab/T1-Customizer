# CI

The GitHub Actions workflow (`.github/workflows/build.yml`) is a simple
build-and-test pipeline for this fork:

- Linux: install GTK 3 development packages, then `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo test` in `rust/`.
- Windows: `cargo build --release` in `rust/`.

There are no self-hosted runners, no Unity packaging, and no device
emulation jobs in this fork. Hardware behaviour is not covered by CI.
