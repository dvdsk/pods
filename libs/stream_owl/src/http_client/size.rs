use http::{self, header, HeaderValue, StatusCode};
use hyper::body::Incoming;
use hyper::Response;

#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct Size {
    pub(crate) bytes: Option<u64>,
}

impl Size {
    #[tracing::instrument(level = "debug", ret)]
    pub(crate) fn update_from_headers(&mut self, response: &Response<Incoming>) {
        let headers = response.headers();

        if response.status() == StatusCode::OK {
            if let Some(content_length) = headers
                .get(header::CONTENT_LENGTH)
                .map(HeaderValue::to_str)
                .and_then(Result::ok)
                .map(|len| u64::from_str_radix(len, 10))
                .and_then(Result::ok)
            {
                self.bytes = Some(content_length)
            }
        } else if response.status() == StatusCode::PARTIAL_CONTENT {
            if let Some(range_total) = headers
                .get(header::CONTENT_RANGE)
                .map(HeaderValue::to_str)
                .and_then(Result::ok)
                .filter(|range| range.starts_with("bytes"))
                .and_then(|range| range.rsplit_once("/"))
                .map(|(_, total)| u64::from_str_radix(total, 10))
                .and_then(Result::ok)
            {
                self.bytes = Some(range_total)
            }
        } else {
            self.bytes = None
        }
    }
}
