use futures_x_io::{
    tokio_io::{AsyncRead, AsyncWrite},
    tokio_io_util::AsyncWriteExt,
};
use futures_x_io_timeoutable::tokio_io::rw::AsyncReadWithTimeoutExt;

//
use std::ops::Deref;
use std::pin::Pin;

fn pin_write_future<P>(write_future: P) -> Pin<P>
where
    P: Deref,
{
    unsafe { Pin::new_unchecked(write_future) }
}

//
#[path = "transport.rs"]
pub mod transport;
