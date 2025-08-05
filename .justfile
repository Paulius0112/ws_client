
build:
  cargo build

bench:
  cargo bench

server:
  RUST_LOG=trace cargo run --example server

run:
  RUST_LOG=info cargo run

check:
  cargo check
  cargo fmt --all -- --check
  cargo clippy --all-targets

fix:
  cargo clippy --allow-dirty --allow-staged --fix
  cargo fmt --all