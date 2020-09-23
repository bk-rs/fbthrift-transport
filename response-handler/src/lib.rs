use std::io;

pub trait ResponseHandler: Clone {
    fn try_make_response_bytes(
        &self,
        request_bytes: &[u8],
    ) -> io::Result<(Vec<u8>, Option<Vec<u8>>)>;

    fn parse_response_bytes(&self, name: &[u8], response_bytes: &[u8])
        -> io::Result<Option<usize>>;
}

#[derive(Clone)]
pub struct DefaultResponseHandler;

impl ResponseHandler for DefaultResponseHandler {
    fn try_make_response_bytes(
        &self,
        _request_bytes: &[u8],
    ) -> io::Result<(Vec<u8>, Option<Vec<u8>>)> {
        Ok((vec![], None))
    }

    fn parse_response_bytes(
        &self,
        _name: &[u8],
        response_bytes: &[u8],
    ) -> io::Result<Option<usize>> {
        Ok(Some(response_bytes.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    #[test]
    fn with_default_response_handler() -> io::Result<()> {
        let h = DefaultResponseHandler;

        match h.try_make_response_bytes(&b""[..]) {
            Ok((i, res_buf)) => {
                assert_eq!(i, b"");
                assert_eq!(res_buf, None);
            }
            Err(err) => assert!(false, err),
        }

        match h.parse_response_bytes(b"", &b"foo"[..]) {
            Ok(n) => {
                assert_eq!(n, Some(3));
            }
            Err(err) => assert!(false, err),
        }

        Ok(())
    }
}
