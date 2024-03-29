use core::{
    ffi::CStr,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use std::{
    io::{Cursor, Error as IoError, ErrorKind as IoErrorKind},
    sync::{Arc, Mutex},
};

use async_sleep::{AsyncReadWithTimeoutExt as _, Sleepble};
use bytes::{Bytes, BytesMut};
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use fbthrift_transport_response_handler::ResponseHandler;
use futures_util::{
    future::BoxFuture,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt as _},
    ready,
};

use crate::configuration::AsyncTransportConfiguration;

//
#[derive(Debug, Clone, Default)]
pub struct AsyncTransportRpcOptions {}

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

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        Self::EncBuf::with_capacity(cap)
    }
}

impl<S, SLEEP, H> Transport for AsyncTransport<S, SLEEP, H>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
    SLEEP: Sleepble + Send + Sync + 'static,
    H: ResponseHandler + Unpin + Send + Sync + 'static,
{
    type RpcOptions = AsyncTransportRpcOptions;

    fn call(
        &self,
        service_name: &'static CStr,
        fn_name: &'static CStr,
        req: FramingEncodedFinal<Self>,
        rpc_options: Self::RpcOptions,
    ) -> BoxFuture<'static, anyhow::Result<FramingDecoded<Self>>> {
        Pin::from(Box::new(Call::<S, SLEEP, H>::new(
            self.stream.clone(),
            service_name,
            fn_name,
            req,
            rpc_options,
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
    service_name: &'static CStr,
    fn_name: &'static CStr,
    req: FramingEncodedFinal<AsyncTransport<S, SLEEP, H>>,
    #[allow(dead_code)]
    rpc_options: AsyncTransportRpcOptions,
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
        service_name: &'static CStr,
        fn_name: &'static CStr,
        req: FramingEncodedFinal<AsyncTransport<S, SLEEP, H>>,
        rpc_options: AsyncTransportRpcOptions,
        configuration: AsyncTransportConfiguration<H>,
    ) -> Self {
        let max_buf_size = configuration.get_max_buf_size();

        Self {
            stream,
            service_name,
            fn_name,
            req,
            rpc_options,
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
            let mut write_future = stream.write_all(&req[..]);
            ready!(Pin::new(&mut write_future).poll(cx))?;

            this.state = CallState::Writed;
        }

        let static_res_buf = configuration
            .response_handler
            .try_make_static_response_bytes(
                service_name.to_bytes(),
                fn_name.to_bytes(),
                &req[..],
            )?;
        if let Some(static_res_buf) = static_res_buf {
            debug_assert!(buf_storage.is_empty(), "The buf_storage should empty");
            return Poll::Ready(Ok(Cursor::new(Bytes::from(static_res_buf))));
        }

        let mut buf = vec![0u8; configuration.get_buf_size()];
        let n_de;
        loop {
            let mut read_future =
                stream.read_with_timeout::<SLEEP>(&mut buf, configuration.get_read_timeout());
            let n = ready!(Pin::new(&mut read_future).poll(cx))?;

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
