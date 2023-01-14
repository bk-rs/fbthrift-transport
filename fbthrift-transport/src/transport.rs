use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use std::{
    io::{Cursor, Error as IoError, ErrorKind as IoErrorKind},
    sync::{Arc, Mutex},
};

use bytes::{Buf, Bytes, BytesMut};
use const_cstr::ConstCStr;
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use fbthrift_transport_response_handler::{DefaultResponseHandler, ResponseHandler};
use futures_core::ready;

use crate::configuration::{AsyncTransportConfiguration, DefaultAsyncTransportConfiguration};

use super::{pin_write_future, AsyncRead, AsyncReadWithTimeoutExt, AsyncWrite, AsyncWriteExt};

pub struct AsyncTransport<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler,
{
    stream: Arc<Mutex<S>>,
    configuration: AsyncTransportConfiguration<H>,
}

impl<S, H> AsyncTransport<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler + Unpin,
{
    pub fn new(stream: S, configuration: AsyncTransportConfiguration<H>) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
            configuration,
        }
    }
}

impl<S> AsyncTransport<S, DefaultResponseHandler>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn with_default_configuration(stream: S) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
            configuration: DefaultAsyncTransportConfiguration::default(),
        }
    }
}

impl<S, H> Framing for AsyncTransport<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler,
{
    type EncBuf = BytesMut;
    type DecBuf = Cursor<Bytes>;
    type Meta = ();

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        Self::EncBuf::with_capacity(cap)
    }

    fn get_meta(&self) {}
}

impl<S, H> Transport for AsyncTransport<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    H: ResponseHandler + Unpin + Send + 'static,
{
    fn call(
        &self,
        service_name: &ConstCStr,
        fn_name: &ConstCStr,
        req: FramingEncodedFinal<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<FramingDecoded<Self>, anyhow::Error>> + Send + 'static>>
    {
        Pin::from(Box::new(Call::new(
            self.stream.clone(),
            service_name.to_owned(),
            fn_name.to_owned(),
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

struct Call<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler,
{
    stream: Arc<Mutex<S>>,
    service_name: ConstCStr,
    fn_name: ConstCStr,
    req: FramingEncodedFinal<AsyncTransport<S, H>>,
    configuration: AsyncTransportConfiguration<H>,
    //
    state: CallState,
    buf_storage: Vec<u8>,
    parsed_response_bytes_count: u8,
}

impl<S, H> Call<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler,
{
    fn new(
        stream: Arc<Mutex<S>>,
        service_name: ConstCStr,
        fn_name: ConstCStr,
        req: FramingEncodedFinal<AsyncTransport<S, H>>,
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

impl<S, H> Future for Call<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler + Unpin,
{
    type Output = Result<FramingDecoded<AsyncTransport<S, H>>, anyhow::Error>;

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

            ready!(pin_write_future(&mut write_future).poll(cx))?;

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
            let mut read_future =
                stream.read_with_timeout(&mut buf, configuration.get_read_timeout());
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

#[cfg(all(feature = "futures_io", not(feature = "tokio_io"),))]
#[path = "transport_call_future_tests_with_futures_io.rs"]
#[cfg(test)]
mod transport_call_future_tests_with_futures_io;

#[cfg(all(not(feature = "futures_io"), feature = "tokio_io",))]
#[path = "transport_call_future_tests_with_tokio_io.rs"]
#[cfg(test)]
mod transport_call_future_tests_with_tokio_io;