cfg_if::cfg_if! {
    if #[cfg(all(feature = "futures_io", not(feature = "tokio_io")))] {
        pub mod transport;
        pub use transport::AsyncTransport;
    } else if #[cfg(all(not(feature = "futures_io"), feature = "tokio_io"))] {
        pub mod transport;
        pub use transport::AsyncTransport;
    }
}

pub mod configuration;
pub use configuration::AsyncTransportConfiguration;
