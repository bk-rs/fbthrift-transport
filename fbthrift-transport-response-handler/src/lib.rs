use core::ffi::CStr;
use std::io::Error as IoError;

//
pub trait ResponseHandler: Clone {
    fn name(&self) -> Option<&str> {
        None
    }

    fn try_make_static_response_bytes(
        &mut self,
        service_name: &'static CStr,
        fn_name: &'static CStr,
        request_bytes: &[u8],
    ) -> Result<Option<Vec<u8>>, IoError>;

    fn parse_response_bytes(&mut self, response_bytes: &[u8]) -> Result<Option<usize>, IoError>;
}

//
#[derive(Debug, Clone, Copy)]
pub struct MockResponseHandler;

impl ResponseHandler for MockResponseHandler {
    fn name(&self) -> Option<&str> {
        Some("Mock")
    }

    fn try_make_static_response_bytes(
        &mut self,
        _service_name: &'static CStr,
        _fn_name: &'static CStr,
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
    fn test_mock_response_handler() -> Result<(), Box<dyn std::error::Error>> {
        let mut h = MockResponseHandler;

        assert_eq!(
            h.try_make_static_response_bytes(
                CStr::from_bytes_with_nul(b"my_service\0").expect(""),
                CStr::from_bytes_with_nul(b"my_fn\0").expect(""),
                &b""[..]
            )?,
            None
        );

        assert_eq!(h.parse_response_bytes(&b"foo"[..])?, Some(3));

        Ok(())
    }
}
