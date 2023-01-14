use std::io::Error as IoError;

pub trait ResponseHandler: Clone {
    fn try_make_static_response_bytes(
        &mut self,
        service_name: &'static str,
        fn_name: &'static str,
        request_bytes: &[u8],
    ) -> Result<Option<Vec<u8>>, IoError>;

    fn parse_response_bytes(&mut self, response_bytes: &[u8]) -> Result<Option<usize>, IoError>;
}

#[derive(Clone)]
pub struct DefaultResponseHandler;

impl ResponseHandler for DefaultResponseHandler {
    fn try_make_static_response_bytes(
        &mut self,
        _service_name: &'static str,
        _fn_name: &'static str,
        _request_bytes: &[u8],
    ) -> Result<Option<Vec<u8>>, IoError> {
        Ok(None)
    }

    fn parse_response_bytes(&mut self, response_bytes: &[u8]) -> Result<Option<usize>, IoError> {
        Ok(Some(response_bytes.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_default_response_handler() -> Result<(), Box<dyn std::error::Error>> {
        let mut h = DefaultResponseHandler;

        assert_eq!(
            h.try_make_static_response_bytes("my_service", "my_fn", &b""[..])?,
            None
        );

        assert_eq!(h.parse_response_bytes(&b"foo"[..])?, Some(3));

        Ok(())
    }
}
