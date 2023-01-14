use std::io::Error as IoError;

//
pub type TokioTcpStream = async_compat::Compat<tokio::net::TcpStream>;
pub type TokioSleep = async_sleep::impl_tokio::Sleep;

//
pub async fn tcp_connect<A: tokio::net::ToSocketAddrs>(addr: A) -> Result<TokioTcpStream, IoError> {
    tokio::net::TcpStream::connect(addr)
        .await
        .map(async_compat::Compat::new)
}
