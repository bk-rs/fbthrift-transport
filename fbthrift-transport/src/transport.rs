use core::{
    future::Future,
    marker::PhantomData,
    ops::DerefMut as _,
    pin::Pin,
    task::{Context, Poll},
};
use std::{
    io::{Cursor, Error as IoError, ErrorKind as IoErrorKind},
    sync::{Arc, Mutex},
};

use async_sleep::Sleepble;
use bytes::{Buf as _, Bytes, BytesMut};
use const_cstr::ConstCStr;
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use fbthrift_transport_response_handler::ResponseHandler;
use futures_util::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt as _},
    ready,
};

use crate::configuration::AsyncTransportConfiguration;

//
pub struct AsyncTransport<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    H: ResponseHandler + Unpin,
{
    stream: Arc<Mutex<S>>,
    configuration: AsyncTransportConfiguration<H>,
    phantom: PhantomData<SLEEP>,
}

impl<S, SLEEP, H> AsyncTransport<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    H: ResponseHandler + Unpin,
{
    pub fn new(stream: S, configuration: AsyncTransportConfiguration<H>) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
            configuration,
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "impl_tokio")]
impl<H> AsyncTransport<crate::impl_tokio::TokioTcpStream, crate::impl_tokio::TokioSleep, H>
where
    H: ResponseHandler + Unpin,
{
    pub async fn with_tokio_tcp_connect<A: tokio::net::ToSocketAddrs>(
        addr: A,
        configuration: AsyncTransportConfiguration<H>,
    ) -> Result<Self, IoError> {
        let stream = crate::impl_tokio::tcp_connect(addr).await?;

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            configuration,
            phantom: PhantomData,
        })
    }
}

#[cfg(feature = "impl_async_io")]
impl<H>
    AsyncTransport<crate::impl_async_io::AsyncIoTcpStream, crate::impl_async_io::AsyncIoSleep, H>
where
    H: ResponseHandler + Unpin,
{
    pub async fn with_async_io_tcp_connect<A: Into<std::net::SocketAddr>>(
        addr: A,
        configuration: AsyncTransportConfiguration<H>,
    ) -> Result<Self, IoError> {
        let stream = crate::impl_async_io::tcp_connect(addr).await?;

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            configuration,
            phantom: PhantomData,
        })
    }
}

//
impl<S, SLEEP, H> Framing for AsyncTransport<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    H: ResponseHandler + Unpin,
{
    type EncBuf = BytesMut;
    type DecBuf = Cursor<Bytes>;
    type Meta = ();

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        Self::EncBuf::with_capacity(cap)
    }

    fn get_meta(&self) {}
}

impl<S, SLEEP, H> Transport for AsyncTransport<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    SLEEP: Sleepble + Send + 'static,
    H: ResponseHandler + Unpin + Send + 'static,
{
    fn call(
        &self,
        service_name: &ConstCStr,
        fn_name: &ConstCStr,
        req: FramingEncodedFinal<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<FramingDecoded<Self>, anyhow::Error>> + Send + 'static>>
    {
        Pin::from(Box::new(Call::<S, SLEEP, H>::new(
            self.stream.clone(),
            service_name.to_owned(),
            fn_name.to_owned(),
            req,
            self.configuration.clone(),
        )))
    }
}

//
#[derive(PartialEq, PartialOrd)]
enum CallState {
    Pending,
    Writed,
}

pub struct Call<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    H: ResponseHandler + Unpin,
{
    stream: Arc<Mutex<S>>,
    service_name: ConstCStr,
    fn_name: ConstCStr,
    req: FramingEncodedFinal<AsyncTransport<S, SLEEP, H>>,
    configuration: AsyncTransportConfiguration<H>,
    //
    state: CallState,
    buf_storage: Vec<u8>,
    parsed_response_bytes_count: u8,
}

impl<S, SLEEP, H> Call<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    H: ResponseHandler + Unpin,
{
    pub fn new(
        stream: Arc<Mutex<S>>,
        service_name: ConstCStr,
        fn_name: ConstCStr,
        req: FramingEncodedFinal<AsyncTransport<S, SLEEP, H>>,
        configuration: AsyncTransportConfiguration<H>,
    ) -> Self {
        let max_buf_size = configuration.get_max_buf_size();

        Self {
            stream,
            service_name,
            fn_name,
            req,
            configuration,
            state: CallState::Pending,
            buf_storage: Vec::with_capacity(max_buf_size),
            parsed_response_bytes_count: 0,
        }
    }
}

impl<S, SLEEP, H> Future for Call<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    H: ResponseHandler + Unpin,
{
    type Output = Result<FramingDecoded<AsyncTransport<S, SLEEP, H>>, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        let stream = &mut match this.stream.lock() {
            Ok(stream) => stream,
            Err(err) => {
                return Poll::Ready(Err(IoError::new(IoErrorKind::Other, err.to_string()).into()))
            }
        };
        let service_name = &this.service_name;
        let fn_name = &this.fn_name;
        let req = &this.req;
        let configuration = &mut this.configuration;
        let buf_storage = &mut this.buf_storage;
        let parsed_response_bytes_count = &mut this.parsed_response_bytes_count;

        if this.state < CallState::Writed {
            let req_bytes = req.bytes();
            let mut write_future = stream.write_all(req_bytes);
            ready!(Pin::new(&mut write_future).poll(cx))?;

            this.state = CallState::Writed;
        }

        let static_res_buf = configuration
            .response_handler
            .try_make_static_response_bytes(service_name.to_str(), fn_name.to_str(), req.bytes())?;
        if let Some(static_res_buf) = static_res_buf {
            debug_assert!(buf_storage.is_empty(), "The buf_storage should empty");
            return Poll::Ready(Ok(Cursor::new(Bytes::from(static_res_buf))));
        }

        let mut buf = vec![0u8; configuration.get_buf_size()];
        let n_de;
        loop {
            let read_future = async_read_poll(
                stream.deref_mut(),
                &mut buf,
                &mut SLEEP::sleep(configuration.get_read_timeout()).wait(),
                cx,
            );
            let n = ready!(read_future)?;

            if n == 0 {
                *parsed_response_bytes_count += 1;
                if *parsed_response_bytes_count > configuration.get_max_parse_response_bytes_count()
                {
                    return Poll::Ready(Err(IoError::new(
                        IoErrorKind::Other,
                        "Reach max parse response bytes count",
                    )
                    .into()));
                }
                continue;
            }

            buf_storage.extend_from_slice(&buf[..n]);

            if let Some(n) = configuration
                .response_handler
                .parse_response_bytes(buf_storage)?
            {
                n_de = n;
                break;
            } else {
                if buf_storage.len() >= configuration.get_max_buf_size() {
                    return Poll::Ready(Err(IoError::new(
                        IoErrorKind::Other,
                        "Reach max buffer size",
                    )
                    .into()));
                }

                *parsed_response_bytes_count += 1;
                if *parsed_response_bytes_count > configuration.get_max_parse_response_bytes_count()
                {
                    return Poll::Ready(Err(IoError::new(
                        IoErrorKind::Other,
                        "Reach max parse response bytes count",
                    )
                    .into()));
                }
            }
        }

        Poll::Ready(Ok(Cursor::new(Bytes::from(buf_storage[..n_de].to_vec()))))
    }
}

fn async_read_poll<R: AsyncRead + Unpin>(
    reader: &mut R,
    buf: &mut [u8],
    delay: &mut Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    cx: &mut Context<'_>,
) -> Poll<Result<usize, IoError>> {
    let poll_ret = Pin::new(reader).poll_read(cx, buf);

    match poll_ret {
        Poll::Ready(ret) => Poll::Ready(ret),
        Poll::Pending => match delay.as_mut().poll(cx) {
            Poll::Ready(_) => Poll::Ready(Err(IoError::new(IoErrorKind::TimedOut, "read timeout"))),
            Poll::Pending => Poll::Pending,
        },
    }
}
