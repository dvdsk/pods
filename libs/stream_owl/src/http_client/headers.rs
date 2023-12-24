use std::num::ParseIntError;
use std::ops::Range;

use axum::response::Response;
use http::header;
use http::header::ToStrError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("")]
    CorruptHeader(ToStrError),
    #[error("")]
    ContentLengthNotNumber(ParseIntError),
    #[error("")]
    MissingContentRange,
    #[error("")]
    RangeNotBytes,
    #[error("")]
    InvalidRangeHeader,
    #[error("")]
    MissingRangeDelimiter,
    #[error("")]
    RangeStartNotNumber(ParseIntError),
    #[error("")]
    RangeStopNotNumber(ParseIntError),
    #[error("")]
    RangeStartSmallerThenEnd,
}

pub fn content_length<T>(response: &Response<T>) -> Result<Option<u64>, Error> {
    let headers = response.headers();
    headers
        .get(header::CONTENT_LENGTH)
        .map(|header| {
            header
                .to_str()
                .map_err(Error::CorruptHeader)?
                .parse()
                .map_err(Error::ContentLengthNotNumber)
        })
        .transpose()
}

fn range_and_total<T>(response: &Response<T>) -> Result<(&str, &str), Error> {
    let headers = response.headers();
    headers
        .get(header::CONTENT_RANGE)
        .ok_or(Error::MissingContentRange)?
        .to_str()
        .map_err(Error::CorruptHeader)?
        .strip_prefix("bytes ")
        .ok_or(Error::RangeNotBytes)?
        .rsplit_once("/")
        .ok_or(Error::InvalidRangeHeader)
}

pub fn range_content_length<T>(response: &Response<T>) -> Result<Option<u64>, Error> {
    let content_length = range_and_total(response)?.1;
    if content_length == "*" {
        Ok(None)
    } else {
        content_length
            .parse()
            .map(Some)
            .map_err(Error::ContentLengthNotNumber)
    }
}

pub fn range<T>(response: &Response<T>) -> Result<Option<Range<u64>>, Error> {
    let range = range_and_total(response)?.0;
    if range == "*" {
        return Ok(None);
    }

    let (start, stop) = range.split_once("-").ok_or(Error::MissingRangeDelimiter)?;
    let start = start.parse().map_err(Error::RangeStartNotNumber)?;
    let stop = stop.parse().map_err(Error::RangeStartNotNumber)?;
    if stop < start {
        Err(Error::RangeStartSmallerThenEnd)
    } else {
        Ok(Some(start..stop))
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;

    #[test]
    fn test_range() {
        fn test_response<'a>(key: &'static str, val: &'a str) -> hyper::Response<&'static str> {
            let mut input = hyper::Response::new("");
            *input.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
            let headers = input.headers_mut();
            headers.insert(key, val.try_into().unwrap());
            input
        }

        for (testcase, expected) in [("content-range", "bytes */10000", None)]
            .map(|(key, val, expected)| (test_response(key, val), expected))
        {
            let expected: Option<Range<u64>> = expected;
            assert_eq!(range(&testcase).unwrap(), expected)
        }
    }
}
