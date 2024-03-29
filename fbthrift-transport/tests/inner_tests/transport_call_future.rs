use super::{block_on, Sleep};

use core::ffi::CStr;
use std::{
    io::Error as IoError,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use fbthrift_transport::{transport::Call, AsyncTransportConfiguration};
use fbthrift_transport_response_handler::ResponseHandler;
use futures_util::io::Cursor;

#[test]
fn call_with_static_res() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Clone)]
    pub struct FooResponseHandler;

    impl ResponseHandler for FooResponseHandler {
        fn try_make_static_response_bytes(
            &mut self,
            _service_name: &'static [u8],
            _fn_name: &'static [u8],
            request_bytes: &[u8],
        ) -> Result<Option<Vec<u8>>, IoError> {
            Ok(if request_bytes == b"static" {
                Some(b"bar".to_vec())
            } else {
                None
            })
        }

        fn parse_response_bytes(
            &mut self,
            _response_bytes: &[u8],
        ) -> Result<Option<usize>, IoError> {
            unimplemented!()
        }
    }

    block_on(async {
        let mut buf = b"1234567890".to_vec();
        let cursor = Cursor::new(&mut buf);
        let stream = Arc::new(Mutex::new(cursor));
        let c = AsyncTransportConfiguration::new(FooResponseHandler);

        //
        let req = Bytes::from("static");
        let call = Call::<_, Sleep, _>::new(
            stream.clone(),
            CStr::from_bytes_with_nul(b"my_service\0").expect(""),
            CStr::from_bytes_with_nul(b"my_fn\0").expect(""),
            req,
            Default::default(),
            c.clone(),
        );

        let out = call.await.expect("");
        assert_eq!(out.into_inner(), Bytes::from("bar"));

        assert_eq!(stream.lock().expect("").get_ref(), &b"static7890");

        Ok(())
    })
}

#[test]
fn call_with_dynamic_res() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Clone)]
    pub struct FooResponseHandler;

    impl ResponseHandler for FooResponseHandler {
        fn try_make_static_response_bytes(
            &mut self,
            _service_name: &'static [u8],
            _fn_name: &'static [u8],
            request_bytes: &[u8],
        ) -> Result<Option<Vec<u8>>, IoError> {
            Ok(if request_bytes == b"dynamic" {
                None
            } else {
                unimplemented!()
            })
        }

        fn parse_response_bytes(
            &mut self,
            response_bytes: &[u8],
        ) -> Result<Option<usize>, IoError> {
            if response_bytes == b"89012" {
                Ok(Some(2))
            } else {
                unimplemented!()
            }
        }
    }

    block_on(async {
        let mut buf = b"123456789012".to_vec();
        let cursor = Cursor::new(&mut buf);
        let stream = Arc::new(Mutex::new(cursor));
        let c = AsyncTransportConfiguration::new(FooResponseHandler);

        //
        let req = Bytes::from("dynamic");
        let call = Call::<_, Sleep, _>::new(
            stream.clone(),
            CStr::from_bytes_with_nul(b"my_service\0").expect(""),
            CStr::from_bytes_with_nul(b"my_fn\0").expect(""),
            req,
            Default::default(),
            c.clone(),
        );

        let out = call.await.expect("");
        assert_eq!(out.into_inner(), Bytes::from("89"));

        assert_eq!(stream.lock().expect("").get_ref(), &b"dynamic89012");

        Ok(())
    })
}

#[test]
fn call_with_dynamic_res_and_less_buf_size() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Clone)]
    pub struct FooResponseHandler;

    impl ResponseHandler for FooResponseHandler {
        fn try_make_static_response_bytes(
            &mut self,
            _service_name: &'static [u8],
            _fn_name: &'static [u8],
            request_bytes: &[u8],
        ) -> Result<Option<Vec<u8>>, IoError> {
            Ok(if request_bytes == b"dynamic" {
                None
            } else {
                unimplemented!()
            })
        }

        fn parse_response_bytes(
            &mut self,
            response_bytes: &[u8],
        ) -> Result<Option<usize>, IoError> {
            match response_bytes {
                b"8" | b"89" | b"890" | b"8901" => Ok(None),
                b"89012" => Ok(Some(4)),
                _ => unimplemented!(),
            }
        }
    }

    block_on(async {
        let mut buf = b"123456789012".to_vec();
        let cursor = Cursor::new(&mut buf);
        let stream = Arc::new(Mutex::new(cursor));
        let mut c = AsyncTransportConfiguration::new(FooResponseHandler);
        c.set_buf_size(1);
        c.set_max_parse_response_bytes_count(99);

        //
        let req = Bytes::from("dynamic");
        let call = Call::<_, Sleep, _>::new(
            stream.clone(),
            CStr::from_bytes_with_nul(b"my_service\0").expect(""),
            CStr::from_bytes_with_nul(b"my_fn\0").expect(""),
            req,
            Default::default(),
            c.clone(),
        );

        let out = call.await.expect("");
        assert_eq!(out.into_inner(), Bytes::from("8901"));

        assert_eq!(stream.lock().expect("").get_ref(), &b"dynamic89012");

        Ok(())
    })
}

#[test]
fn call_with_dynamic_res_and_less_max_buf_size() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Clone)]
    pub struct FooResponseHandler;

    impl ResponseHandler for FooResponseHandler {
        fn try_make_static_response_bytes(
            &mut self,
            _service_name: &'static [u8],
            _fn_name: &'static [u8],
            request_bytes: &[u8],
        ) -> Result<Option<Vec<u8>>, IoError> {
            Ok(if request_bytes == b"dynamic" {
                None
            } else {
                unimplemented!()
            })
        }

        fn parse_response_bytes(
            &mut self,
            response_bytes: &[u8],
        ) -> Result<Option<usize>, IoError> {
            match response_bytes {
                b"8" | b"89" | b"890" | b"8901" => Ok(None),
                b"89012" => Ok(Some(4)),
                _ => unimplemented!(),
            }
        }
    }

    block_on(async {
        let mut buf = b"123456789012".to_vec();
        let cursor = Cursor::new(&mut buf);
        let stream = Arc::new(Mutex::new(cursor));
        let mut c = AsyncTransportConfiguration::new(FooResponseHandler);
        c.set_buf_size(1);
        c.set_max_buf_size(3);

        //
        let req = Bytes::from("dynamic");
        let call = Call::<_, Sleep, _>::new(
            stream.clone(),
            CStr::from_bytes_with_nul(b"my_service\0").expect(""),
            CStr::from_bytes_with_nul(b"my_fn\0").expect(""),
            req,
            Default::default(),
            c.clone(),
        );

        match call.await {
            Ok(_) => panic!(),
            Err(err) => {
                assert!(err.to_string() == "Reach max buffer size");
            }
        }

        assert_eq!(stream.lock().expect("").get_ref(), &b"dynamic89012");

        Ok(())
    })
}

#[test]
fn call_with_dynamic_res_and_too_many_read() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Clone)]
    pub struct FooResponseHandler;

    impl ResponseHandler for FooResponseHandler {
        fn try_make_static_response_bytes(
            &mut self,
            _service_name: &'static [u8],
            _fn_name: &'static [u8],
            request_bytes: &[u8],
        ) -> Result<Option<Vec<u8>>, IoError> {
            Ok(if request_bytes == b"dynamic" {
                None
            } else {
                unimplemented!()
            })
        }

        fn parse_response_bytes(
            &mut self,
            response_bytes: &[u8],
        ) -> Result<Option<usize>, IoError> {
            if response_bytes == b"" {
                Ok(None)
            } else {
                unimplemented!()
            }
        }
    }

    block_on(async {
        let mut buf = b"".to_vec();
        let cursor = Cursor::new(&mut buf);
        let stream = Arc::new(Mutex::new(cursor));
        let c = AsyncTransportConfiguration::new(FooResponseHandler);

        //
        let req = Bytes::from("dynamic");
        let call = Call::<_, Sleep, _>::new(
            stream.clone(),
            CStr::from_bytes_with_nul(b"my_service\0").expect(""),
            CStr::from_bytes_with_nul(b"my_fn\0").expect(""),
            req,
            Default::default(),
            c.clone(),
        );

        match call.await {
            Ok(_) => panic!(),
            Err(err) => {
                assert!(err.to_string() == "Reach max parse response bytes count");
            }
        }

        assert_eq!(stream.lock().expect("").get_ref(), &b"dynamic");

        Ok(())
    })
}
