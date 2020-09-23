use std::io;
use std::time::Duration;

#[derive(Clone)]
pub struct AsyncTransportConfiguration<I, FDQ, FDS>
where
    FDQ: Fn(&[u8]) -> io::Result<(I, Option<Vec<u8>>)>,
    FDS: Fn(I, &[u8]) -> io::Result<Option<usize>>,
{
    buf_size: Option<usize>,
    max_buf_size: Option<usize>,
    read_timeout: Option<Duration>,
    pub(crate) de_req_bytes: FDQ,
    pub(crate) de_res_bytes: FDS,
}

impl<I, FDQ, FDS> AsyncTransportConfiguration<I, FDQ, FDS>
where
    FDQ: Fn(&[u8]) -> io::Result<(I, Option<Vec<u8>>)>,
    FDS: Fn(I, &[u8]) -> io::Result<Option<usize>>,
{
    pub fn new(de_req_bytes: FDQ, de_res_bytes: FDS) -> Self {
        Self {
            buf_size: None,
            max_buf_size: None,
            read_timeout: None,
            de_req_bytes,
            de_res_bytes,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_new() -> io::Result<()> {
        let c =
            AsyncTransportConfiguration::new(|_| Ok(("", None)), |_, bytes| Ok(Some(bytes.len())));

        assert_eq!(c.get_buf_size(), 1024);
        assert_eq!(c.get_read_timeout(), Duration::from_secs(5));

        match (c.de_req_bytes)(&b""[..]) {
            Ok((i, res_buf)) => {
                assert_eq!(i, "");
                assert_eq!(res_buf, None);
            }
            Err(err) => assert!(false, err),
        }

        match (c.de_res_bytes)("", &b"foo"[..]) {
            Ok(n) => {
                assert_eq!(n, Some(3));
            }
            Err(err) => assert!(false, err),
        }

        Ok(())
    }
}
