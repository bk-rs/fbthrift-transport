use std::time::Duration;

use fbthrift_transport_response_handler::{DefaultResponseHandler, ResponseHandler};

#[derive(Clone)]
pub struct AsyncTransportConfiguration<H>
where
    H: ResponseHandler,
{
    buf_size: Option<usize>,
    max_buf_size: Option<usize>,
    read_timeout: Option<Duration>,
    max_parse_response_bytes_count: Option<u8>,
    pub(crate) response_handler: H,
}

impl<H> AsyncTransportConfiguration<H>
where
    H: ResponseHandler,
{
    pub fn new(response_handler: H) -> Self {
        Self {
            buf_size: None,
            max_buf_size: None,
            read_timeout: None,
            max_parse_response_bytes_count: None,
            response_handler,
        }
    }

    pub fn set_buf_size(&mut self, size: usize) {
        self.buf_size = Some(size);
    }

    pub fn get_buf_size(&self) -> usize {
        self.buf_size.unwrap_or(1024)
    }

    pub fn set_max_buf_size(&mut self, size: usize) {
        self.max_buf_size = Some(size);
    }

    pub fn get_max_buf_size(&self) -> usize {
        self.max_buf_size.unwrap_or(1024 * 16)
    }

    pub fn set_read_timeout(&mut self, timeout_ms: u32) {
        self.read_timeout = Some(Duration::from_millis(timeout_ms as u64));
    }

    pub fn get_read_timeout(&self) -> Duration {
        self.read_timeout.unwrap_or(Duration::from_secs(5))
    }

    pub fn set_max_parse_response_bytes_count(&mut self, size: u8) {
        self.max_parse_response_bytes_count = Some(size);
    }

    pub fn get_max_parse_response_bytes_count(&self) -> u8 {
        self.max_parse_response_bytes_count.unwrap_or(3)
    }
}

pub type DefaultAsyncTransportConfiguration = AsyncTransportConfiguration<DefaultResponseHandler>;

impl Default for DefaultAsyncTransportConfiguration {
    fn default() -> Self {
        Self::new(DefaultResponseHandler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    #[test]
    fn with_default() -> io::Result<()> {
        let c: DefaultAsyncTransportConfiguration = Default::default();

        assert_eq!(c.get_buf_size(), 1024);
        assert_eq!(c.get_read_timeout(), Duration::from_secs(5));
        assert_eq!(c.get_max_parse_response_bytes_count(), 3);

        Ok(())
    }
}
