#![cfg(feature = "impl_async_io")]

#[cfg(test)]
mod transport_impl_async_io_tests {
    use core::ffi::CStr;
    use std::{
        io::{Error as IoError, ErrorKind as IoErrorKind},
        net::TcpListener,
        sync::Arc,
        thread,
    };

    use bytes::Bytes;
    use fbthrift::Transport as _;

    use async_executor::{Executor, Task};
    use async_io::Async;
    use futures_lite::{
        future::{self, block_on},
        io::{AsyncReadExt as _, AsyncWriteExt as _},
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
            _service_name: &'static CStr,
            _fn_name: &'static CStr,
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
        let ex = Executor::new();
        let ex = Arc::new(ex);

        let ex_with_run_pending = ex.clone();
        thread::spawn(move || block_on(ex_with_run_pending.run(future::pending::<()>())));

        block_on(async move {
            let listen_addr_for_server = TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap();
            let listen_addr_for_client = listen_addr_for_server;

            let server: Task<Result<(), IoError>> = ex.clone().spawn(async move {
                let listener = Async::<TcpListener>::bind(listen_addr_for_server)?;

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

            let client: Task<Result<(), IoError>> = ex.clone().spawn(async move {
                let transport = AsyncTransport::with_async_io_tcp_connect(
                    listen_addr_for_client,
                    AsyncTransportConfiguration::new(MockResponseHandler),
                )
                .await?;

                for n in 0..10_usize {
                    let cursor = transport
                        .call(
                            CStr::from_bytes_with_nul(b"my_service\0").expect(""),
                            CStr::from_bytes_with_nul(b"my_fn\0").expect(""),
                            Bytes::from("abcde"),
                            Default::default(),
                        )
                        .await
                        .map_err(|err| IoError::new(IoErrorKind::Other, err))?;

                    println!("futures_io transport.call {n} {cursor:?}");
                    assert_eq!(cursor.into_inner(), Bytes::from("abcde"));
                }

                Ok(())
            });

            client.await?;
            assert!(server.cancel().await.is_some());

            Ok(())
        })
    }
}

//
//
//
pub type Sleep = fbthrift_transport::impl_async_io::AsyncIoSleep;

fn block_on<T>(future: impl core::future::Future<Output = T>) -> T {
    futures_lite::future::block_on(future)
}

#[cfg(test)]
#[path = "./inner_tests/transport_call_future.rs"]
mod transport_impl_async_io_call_tests;
