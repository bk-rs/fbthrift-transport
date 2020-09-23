use std::future::Future;
use std::io;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use bytes::Buf;
use bytes::{Bytes, BytesMut};
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use futures_core::ready;
use futures_x_io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use futures_x_io_timeoutable::AsyncReadWithTimeoutExt;

use crate::configuration::{AsyncTransportConfiguration, DefaultAsyncTransportConfiguration};
use fbthrift_transport_response_handler::{ResponseHandler, DefaultResponseHandler};

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

struct Call<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler,
{
    stream: Arc<Mutex<S>>,
    req: FramingEncodedFinal<AsyncTransport<S, H>>,
    configuration: AsyncTransportConfiguration<H>,
    //
    state: CallState,
    buf_storage: Vec<u8>,
}

impl<S, H> Call<S, H>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: ResponseHandler,
{
    fn new(
        stream: Arc<Mutex<S>>,
        req: FramingEncodedFinal<AsyncTransport<S, H>>,
        configuration: AsyncTransportConfiguration<H>,
    ) -> Self {
        let max_buf_size = configuration.get_max_buf_size();

        Self {
            stream,
            req,
            configuration,
            state: CallState::Pending,
            buf_storage: Vec::with_capacity(max_buf_size),
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
                return Poll::Ready(Err(
                    io::Error::new(io::ErrorKind::Other, err.to_string()).into()
                ))
            }
        };
        let req = &this.req;
        let configuration = &this.configuration;
        let buf_storage = &mut this.buf_storage;

        if this.state < CallState::Writed {
            let req_bytes = req.bytes();
            let mut write_future = stream.write_all(req_bytes);
            ready!(Pin::new(&mut write_future).poll(cx))?;
            this.state = CallState::Writed;
        }

        let (identity, res_buf) = configuration
            .response_handler
            .try_make_response_bytes(req.bytes())?;
        if let Some(res_buf) = res_buf {
            debug_assert!(buf_storage.is_empty(), "buf_storage should empty");
            return Poll::Ready(Ok(Cursor::new(Bytes::from(res_buf))));
        }

        let mut buf = vec![0u8; configuration.get_buf_size()];
        let n_de;
        loop {
            let mut read_future =
                stream.read_with_timeout(&mut buf, configuration.get_read_timeout());
            let n = ready!(Pin::new(&mut read_future).poll(cx))?;
            buf_storage.extend_from_slice(&buf[..n]);

            if let Some(n) = configuration
                .response_handler
                .parse_response_bytes(identity.clone(), &buf_storage)?
            {
                n_de = n;
                break;
            } else {
                if buf_storage.len() >= configuration.get_max_buf_size() {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Reach max buffer size",
                    )
                    .into()));
                }
            }
        }
        return Poll::Ready(Ok(Cursor::new(Bytes::from(buf_storage[..n_de].to_vec()))));
    }
}

#[cfg(all(feature = "futures_io", not(feature = "tokio_io")))]
#[cfg(test)]
mod tests {
    use super::*;

    use std::panic;

    use futures_lite::future::block_on;
    use futures_lite::io::Cursor;

    #[test]
    fn call_with_static_res() -> io::Result<()> {
        #[derive(Clone)]
        pub struct FooResponseHandler;

        impl ResponseHandler for FooResponseHandler {
            fn try_make_response_bytes(
                &self,
                request_bytes: &[u8],
            ) -> io::Result<(&str, Option<Vec<u8>>)> {
                Ok((
                    "",
                    if request_bytes == b"static" {
                        Some(b"bar".to_vec())
                    } else {
                        None
                    },
                ))
            }

            fn parse_response_bytes(
                &self,
                _identity: &str,
                _response_bytes: &[u8],
            ) -> io::Result<Option<usize>> {
                unimplemented!()
            }
        }

        block_on(async {
            let mut buf = b"1234567890".to_vec();
            let cursor = Cursor::new(&mut buf);
            let stream = Arc::new(Mutex::new(cursor));
            let c = AsyncTransportConfiguration::new(FooResponseHandler);

            //
            let req = Bytes::from("static");
            let call = Call::new(stream.clone(), req, c.clone());

            let out = call.await.expect("");
            assert_eq!(out.into_inner(), Bytes::from("bar"));

            assert_eq!(stream.lock().expect("").get_ref(), &b"static7890");

            Ok(())
        })
    }

    #[test]
    fn call_with_dynamic_res() -> io::Result<()> {
        #[derive(Clone)]
        pub struct FooResponseHandler;

        impl ResponseHandler for FooResponseHandler {
            fn try_make_response_bytes(
                &self,
                request_bytes: &[u8],
            ) -> io::Result<(&str, Option<Vec<u8>>)> {
                Ok((
                    "id1",
                    if request_bytes == b"dynamic" {
                        None
                    } else {
                        unimplemented!()
                    },
                ))
            }

            fn parse_response_bytes(
                &self,
                identity: &str,
                response_bytes: &[u8],
            ) -> io::Result<Option<usize>> {
                if identity == "id1" && response_bytes == b"89012" {
                    Ok(Some(2))
                } else {
                    unimplemented!()
                }
            }
        }

        block_on(async {
            let mut buf = b"123456789012".to_vec();
            let cursor = Cursor::new(&mut buf);
            let stream = Arc::new(Mutex::new(cursor));
            let c = AsyncTransportConfiguration::new(FooResponseHandler);

            //
            let req = Bytes::from("dynamic");
            let call = Call::new(stream.clone(), req, c.clone());

            let out = call.await.expect("");
            assert_eq!(out.into_inner(), Bytes::from("89"));

            assert_eq!(stream.lock().expect("").get_ref(), &b"dynamic89012");

            Ok(())
        })
    }

    #[test]
    fn call_with_dynamic_res_and_less_buf_size() -> io::Result<()> {
        #[derive(Clone)]
        pub struct FooResponseHandler;

        impl ResponseHandler for FooResponseHandler {
            fn try_make_response_bytes(
                &self,
                request_bytes: &[u8],
            ) -> io::Result<(&str, Option<Vec<u8>>)> {
                Ok((
                    "id1",
                    if request_bytes == b"dynamic" {
                        None
                    } else {
                        unimplemented!()
                    },
                ))
            }

            fn parse_response_bytes(
                &self,
                identity: &str,
                response_bytes: &[u8],
            ) -> io::Result<Option<usize>> {
                if identity == "id1" {
                    if response_bytes == b"8" {
                        Ok(None)
                    } else if response_bytes == b"89" {
                        Ok(None)
                    } else if response_bytes == b"890" {
                        Ok(None)
                    } else if response_bytes == b"8901" {
                        Ok(None)
                    } else if response_bytes == b"89012" {
                        Ok(Some(4))
                    } else {
                        unimplemented!()
                    }
                } else {
                    unimplemented!()
                }
            }
        }

        block_on(async {
            let mut buf = b"123456789012".to_vec();
            let cursor = Cursor::new(&mut buf);
            let stream = Arc::new(Mutex::new(cursor));
            let mut c = AsyncTransportConfiguration::new(FooResponseHandler);
            c.set_buf_size(1);

            //
            let req = Bytes::from("dynamic");
            let call = Call::new(stream.clone(), req, c.clone());

            let out = call.await.expect("");
            assert_eq!(out.into_inner(), Bytes::from("8901"));

            assert_eq!(stream.lock().expect("").get_ref(), &b"dynamic89012");

            Ok(())
        })
    }

    #[test]
    fn call_with_dynamic_res_and_less_max_buf_size() -> io::Result<()> {
        #[derive(Clone)]
        pub struct FooResponseHandler;

        impl ResponseHandler for FooResponseHandler {
            fn try_make_response_bytes(
                &self,
                request_bytes: &[u8],
            ) -> io::Result<(&str, Option<Vec<u8>>)> {
                Ok((
                    "id1",
                    if request_bytes == b"dynamic" {
                        None
                    } else {
                        unimplemented!()
                    },
                ))
            }

            fn parse_response_bytes(
                &self,
                identity: &str,
                response_bytes: &[u8],
            ) -> io::Result<Option<usize>> {
                if identity == "id1" {
                    if response_bytes == b"8" {
                        Ok(None)
                    } else if response_bytes == b"89" {
                        Ok(None)
                    } else if response_bytes == b"890" {
                        Ok(None)
                    } else if response_bytes == b"8901" {
                        Ok(None)
                    } else if response_bytes == b"89012" {
                        Ok(Some(4))
                    } else {
                        unimplemented!()
                    }
                } else {
                    unimplemented!()
                }
            }
        }

        block_on(async {
            let mut buf = b"123456789012".to_vec();
            let cursor = Cursor::new(&mut buf);
            let stream = Arc::new(Mutex::new(cursor));
            let mut c = AsyncTransportConfiguration::new(FooResponseHandler);
            c.set_buf_size(1);
            c.set_max_buf_size(3);

            //
            let req = Bytes::from("dynamic");
            let call = Call::new(stream.clone(), req, c.clone());

            match call.await {
                Ok(_) => assert!(false),
                Err(err) => {
                    assert!(err.to_string() == "Reach max buffer size");
                }
            }

            assert_eq!(stream.lock().expect("").get_ref(), &b"dynamic89012");

            Ok(())
        })
    }
}
