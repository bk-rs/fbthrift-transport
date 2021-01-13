#[cfg(all(
    feature = "futures_io",
    not(feature = "tokio02_io"),
    not(feature = "tokio_io"),
))]
use futures_lite::io::Cursor;
#[cfg(all(
    feature = "futures_io",
    not(feature = "tokio02_io"),
    not(feature = "tokio_io"),
))]
fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    futures_lite::future::block_on(future)
}

#[cfg(all(
    feature = "futures_io",
    not(feature = "tokio02_io"),
    not(feature = "tokio_io"),
))]
#[cfg(test)]
#[path = "./inner_tests/transport.rs"]
mod transport_with_futures_io_tests;

//
//
//
#[cfg(all(
    not(feature = "futures_io"),
    feature = "tokio02_io",
    not(feature = "tokio_io"),
))]
use std::io::Cursor;
#[cfg(all(
    not(feature = "futures_io"),
    feature = "tokio02_io",
    not(feature = "tokio_io"),
))]
fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    let mut rt = tokio02::runtime::Runtime::new().unwrap();
    rt.block_on(future)
}

#[cfg(all(
    not(feature = "futures_io"),
    feature = "tokio02_io",
    not(feature = "tokio_io"),
))]
#[cfg(test)]
#[path = "./inner_tests/transport.rs"]
mod transport_with_tokio02_io_tests;

//
//
//
#[cfg(all(
    not(feature = "futures_io"),
    not(feature = "tokio02_io"),
    feature = "tokio_io",
))]
use std::io::Cursor;
#[cfg(all(
    not(feature = "futures_io"),
    not(feature = "tokio02_io"),
    feature = "tokio_io",
))]
fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(future)
}

#[cfg(all(
    not(feature = "futures_io"),
    not(feature = "tokio02_io"),
    feature = "tokio_io",
))]
#[cfg(test)]
#[path = "./inner_tests/transport.rs"]
mod transport_with_tokio_io_tests;
