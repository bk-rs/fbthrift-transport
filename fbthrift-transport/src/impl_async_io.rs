use std::io::Error as IoError;

//
pub type AsyncIoTcpStream = async_io::Async<std::net::TcpStream>;
pub type AsyncIoSleep = async_sleep::impl_async_io::Timer;

//
pub async fn tcp_connect<A: Into<std::net::SocketAddr>>(
    addr: A,
) -> Result<AsyncIoTcpStream, IoError> {
    AsyncIoTcpStream::connect(addr).await
}
