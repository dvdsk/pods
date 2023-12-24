use std::ops::Range;

use http::StatusCode;
use hyper::body::Incoming;

use crate::http_client::headers;

use super::headers::content_length;

#[derive(Debug)]
pub(crate) enum ValidResponse {
    Ok {
        stream: Incoming,
        /// servers are allowed to leave out Content-Length :(
        /// a livestream could leave it out for example
        size: Option<u64>,
    },
    PartialContent {
        stream: Incoming,
        size: Option<u64>,
        range: Range<u64>,
    },
    RangeNotSatisfiable {
        size: Option<u64>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("")]
    InvalidPartialContentHeader(headers::Error),
    #[error("")]
    InvalidRangeNotSatHeader(headers::Error),
    #[error("")]
    InvalidOkHeader(headers::Error),
    #[error("")]
    IncorrectStatus(StatusCode),
    #[error("The http spec does not allow range: * (indicating unsatisfied) with a PARTIAL_CONTENT status")]
    UnsatisfiedRangeInPartialContent,
}

impl TryFrom<hyper::Response<Incoming>> for ValidResponse {
    type Error = Error;

    fn try_from(response: hyper::Response<Incoming>) -> Result<Self, Self::Error> {
        match response.status() {
            StatusCode::OK => Self::check_ok(response),
            StatusCode::PARTIAL_CONTENT => Self::check_partial_content(response),
            StatusCode::RANGE_NOT_SATISFIABLE => Self::check_range_not_sat(response),
            code => Err(Error::IncorrectStatus(code)),
        }
    }
}

impl ValidResponse {
    pub(crate) fn stream_size(&self) -> Option<u64> {
        match self {
            ValidResponse::Ok { size, .. } => *size,
            ValidResponse::PartialContent { size, .. } => *size,
            ValidResponse::RangeNotSatisfiable { size } => *size,
        }
    }

    fn check_ok(response: hyper::Response<Incoming>) -> Result<ValidResponse, Error> {
        let size = content_length(&response).map_err(Error::InvalidOkHeader)?;
        Ok(ValidResponse::Ok {
            stream: response.into_body(),
            size,
        })
    }

    fn check_partial_content(response: hyper::Response<Incoming>) -> Result<ValidResponse, Error> {
        let size =
            headers::range_content_length(&response).map_err(Error::InvalidPartialContentHeader)?;
        let range = headers::range(&response)
            .map_err(Error::InvalidPartialContentHeader)?
            .ok_or(Error::UnsatisfiedRangeInPartialContent)?;

        Ok(ValidResponse::PartialContent {
            size,
            range,
            stream: response.into_body(),
        })
    }

    fn check_range_not_sat(response: http::Response<Incoming>) -> Result<ValidResponse, Error> {
        let size =
            headers::range_content_length(&response).map_err(Error::InvalidRangeNotSatHeader)?;

        Ok(ValidResponse::RangeNotSatisfiable { size })
    }
}
