# fbthrift-transport

* [Cargo package](https://crates.io/crates/fbthrift-transport)

## Examples

### async-std

* [nebula-graph](https://github.com/bk-rs/nebula-rs/blob/master/demos/async-std/src/graph_client.rs)

### tokio

* [nebula-graph](https://github.com/bk-rs/nebula-rs/blob/master/demos/tokio/src/graph_client.rs)

## Dev

```
cargo clippy --all --all-features -- -D clippy::all
cargo +nightly clippy --all --all-features -- -D clippy::all

cargo fmt --all -- --check
```

```
cargo build-all-features
cargo test-all-features -- --nocapture
```
