#![cfg(feature = "impl_tokio")]

#[cfg(test)]
mod transport_impl_tokio_tests {
    use std::io::{Error as IoError, ErrorKind as IoErrorKind};

    use bytes::Bytes;
    use const_cstr::const_cstr;
    use fbthrift::Transport as _;

    use tokio::{
        io::{AsyncReadExt as _, AsyncWriteExt as _},
        net::TcpListener,
        runtime::Runtime,
        task::JoinHandle,
    };

    use fbthrift_transport::{
        fbthrift_transport_response_handler::{MockResponseHandler, ResponseHandler},
        AsyncTransport, AsyncTransportConfiguration,
    };

    #[derive(Clone)]
    pub struct FooResponseHandler;

    impl ResponseHandler for FooResponseHandler {
        fn try_make_static_response_bytes(
            &mut self,
            _service_name: &'static str,
            _fn_name: &'static str,
            _request_bytes: &[u8],
        ) -> Result<Option<Vec<u8>>, IoError> {
            Ok(None)
        }

        fn parse_response_bytes(
            &mut self,
            response_bytes: &[u8],
        ) -> Result<Option<usize>, IoError> {
            Ok(if response_bytes == b"abcde" {
                Some(5)
            } else {
                None
            })
        }
    }

    #[test]
    fn simple() -> Result<(), Box<dyn std::error::Error>> {
        let rt = Runtime::new().unwrap();

        let listener: Result<TcpListener, IoError> =
            rt.block_on(async move { TcpListener::bind("127.0.0.1:0").await });

        let listener = listener?;

        let listen_addr_for_client = listener.local_addr()?;

        let server: JoinHandle<Result<(), IoError>> = rt.spawn(async move {
            let (mut stream, _) = listener.accept().await?;

            let mut n: usize = 0;
            let mut buf = vec![0; 5];
            loop {
                stream.read_exact(&mut buf).await?;
                stream.write_all(&buf).await?;

                n += 1;
                if n >= 10 {
                    break;
                }
            }

            Ok(())
        });

        let client: Result<(), IoError> = rt.block_on(async move {
            let transport = AsyncTransport::with_tokio_tcp_connect(
                listen_addr_for_client,
                AsyncTransportConfiguration::new(MockResponseHandler),
            )
            .await?;

            for n in 0..10_usize {
                let cursor = transport
                    .call(
                        &const_cstr!("my_service"),
                        &const_cstr!("my_fn"),
                        Bytes::from("abcde"),
                    )
                    .await
                    .map_err(|err| IoError::new(IoErrorKind::Other, err))?;

                println!("transport_impl_tokio transport.call {n} {cursor:?}");

                assert_eq!(cursor.into_inner(), Bytes::from("abcde"));
            }

            Ok(())
        });

        match client {
            Ok(_) => {}
            Err(err) => {
                panic!("{err}");
            }
        }

        rt.block_on(async move {
            assert!(server.await.ok().is_some());
        });

        Ok(())
    }
}

//
//
//
pub type Sleep = fbthrift_transport::impl_tokio::TokioSleep;

fn block_on<T>(future: impl core::future::Future<Output = T>) -> T {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(future)
}

#[cfg(test)]
#[path = "./inner_tests/transport_call_future.rs"]
mod transport_impl_tokio_inner_tests;
