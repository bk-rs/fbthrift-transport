use std::future::Future;
use std::io;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Buf;
use bytes::{Bytes, BytesMut};
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use futures_core::ready;
use futures_x_io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use futures_x_io_timeoutable::AsyncReadWithTimeoutExt;

pub struct AsyncTransport<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream: Arc<Mutex<S>>,
    configuration: AsyncTransportConfiguration,
}

impl<S> AsyncTransport<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: S, configuration: Option<AsyncTransportConfiguration>) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
            configuration: configuration.unwrap_or_default(),
        }
    }
}

impl<S> Framing for AsyncTransport<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type EncBuf = BytesMut;
    type DecBuf = Cursor<Bytes>;
    type Meta = ();

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        Self::EncBuf::with_capacity(cap)
    }

    fn get_meta(&self) {}
}

impl<S> Transport for AsyncTransport<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    fn call(
        &self,
        req: FramingEncodedFinal<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<FramingDecoded<Self>, anyhow::Error>> + Send + 'static>>
    {
        Pin::from(Box::new(Call::new(
            self.stream.clone(),
            req,
            self.configuration.clone(),
        )))
    }
}

#[derive(PartialEq, PartialOrd)]
enum CallState {
    Pending,
    Writed,
}

struct Call<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream: Arc<Mutex<S>>,
    req: FramingEncodedFinal<AsyncTransport<S>>,
    configuration: AsyncTransportConfiguration,
    //
    state: CallState,
}

impl<S> Call<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(
        stream: Arc<Mutex<S>>,
        req: FramingEncodedFinal<AsyncTransport<S>>,
        configuration: AsyncTransportConfiguration,
    ) -> Self {
        Self {
            stream,
            req,
            configuration,
            state: CallState::Pending,
        }
    }
}

impl<S> Future for Call<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Output = Result<FramingDecoded<AsyncTransport<S>>, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        let stream = &mut match this.stream.lock() {
            Ok(stream) => stream,
            Err(err) => {
                return Poll::Ready(Err(
                    io::Error::new(io::ErrorKind::Other, err.to_string()).into()
                ))
            }
        };
        let req = &this.req;
        let configuration = &this.configuration;

        if this.state < CallState::Writed {
            let req_bytes = req.bytes();
            let mut write_future = stream.write_all(req_bytes);
            ready!(Pin::new(&mut write_future).poll(cx))?;
            this.state = CallState::Writed;
        }

        // TODO, loop
        let mut res_buf = vec![0u8; configuration.get_buf_size()];
        let mut read_future =
            stream.read_with_timeout(&mut res_buf, configuration.get_read_timeout());
        ready!(Pin::new(&mut read_future).poll(cx))?;

        Poll::Ready(Ok(Cursor::new(Bytes::from(res_buf))))
    }
}

//
//
//
#[derive(Default, Clone)]
pub struct AsyncTransportConfiguration {
    buf_size: Option<usize>,
    read_timeout: Option<Duration>,
}

impl AsyncTransportConfiguration {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_buf_size(&mut self, size: usize) {
        self.buf_size = Some(size);
    }

    fn get_buf_size(&self) -> usize {
        self.buf_size.unwrap_or(1024)
    }

    pub fn set_read_timeout(&mut self, timeout_ms: u32) {
        self.read_timeout = Some(Duration::from_millis(timeout_ms as u64));
    }

    fn get_read_timeout(&self) -> Duration {
        self.read_timeout.unwrap_or(Duration::from_secs(5))
    }
}
