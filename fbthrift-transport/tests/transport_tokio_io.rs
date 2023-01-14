#[cfg(all(not(feature = "futures_io"), feature = "tokio_io",))]
mod transport_tokio_io_tests {
    use std::{error, io};

    use bytes::Bytes;
    use const_cstr::const_cstr;
    use fbthrift::Transport;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        runtime::Runtime,
        task::JoinHandle,
    };

    use fbthrift_transport::AsyncTransport;
    use fbthrift_transport_response_handler::ResponseHandler;

    #[derive(Clone)]
    pub struct FooResponseHandler;

    impl ResponseHandler for FooResponseHandler {
        fn try_make_static_response_bytes(
            &mut self,
            _service_name: &'static str,
            _fn_name: &'static str,
            _request_bytes: &[u8],
        ) -> io::Result<Option<Vec<u8>>> {
            Ok(None)
        }

        fn parse_response_bytes(&mut self, response_bytes: &[u8]) -> io::Result<Option<usize>> {
            Ok(if response_bytes == b"abcde" {
                Some(5)
            } else {
                None
            })
        }
    }

    #[test]
    fn simple() -> Result<(), Box<dyn error::Error>> {
        let rt = Runtime::new().unwrap();

        let listener: io::Result<TcpListener> =
            rt.block_on(async move { TcpListener::bind("127.0.0.1:0").await });

        let listener = listener?;

        let listen_addr_for_client = listener.local_addr()?;

        let server: JoinHandle<io::Result<()>> = rt.spawn(async move {
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

        let client: io::Result<()> = rt.block_on(async move {
            let stream = TcpStream::connect(listen_addr_for_client).await?;

            let transport = AsyncTransport::with_default_configuration(stream);

            for n in 0..10_usize {
                let cursor = transport
                    .call(
                        &const_cstr!("my_service"),
                        &const_cstr!("my_fn"),
                        Bytes::from("abcde"),
                    )
                    .await
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

                println!("tokio_io transport.call {} {:?}", n, cursor);

                assert_eq!(cursor.into_inner(), Bytes::from("abcde"));
            }

            Ok(())
        });

        match client {
            Ok(_) => {}
            Err(err) => {
                eprintln!("client {:?}", err);
                assert!(false, err);
            }
        }

        server.abort();

        drop(server);

        Ok(())
    }
}
