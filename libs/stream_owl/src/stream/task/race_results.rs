use super::Error;
use crate::http_client::Error as HttpError;
use crate::http_client::{RangeRefused, RangeSupported, StreamingClient};

pub(super) enum Res1 {
    Seek(Option<u64>),
    NewClient(Result<StreamingClient, HttpError>),
}

pub(super) enum Res2 {
    Seek(Option<u64>),
    Write(Result<(), HttpError>),
}

pub(super) enum Res3 {
    Seek(Option<u64>),
    StreamError(Error),
    StreamRangesSupported(RangeSupported),
    StreamRangesRefused(RangeRefused),
    StreamDone,
}

impl From<Result<Option<StreamingClient>, Error>> for Res3 {
    fn from(value: Result<Option<StreamingClient>, Error>) -> Self {
        let client = match value {
            Ok(Some(client)) => client,
            Ok(None) => return Self::StreamDone,
            Err(err) => return Self::StreamError(err),
        };

        match client {
            StreamingClient::RangesSupported(c) => Self::StreamRangesSupported(c),
            StreamingClient::RangesRefused(c) => Self::StreamRangesRefused(c),
        }
    }
}

pub(super) enum Res4 {
    Seek(Option<u64>),
    GetClientError(Error),
    GotRangesSupported(RangeSupported),
    GotRangesRefused(RangeRefused),
}

impl From<Result<StreamingClient, HttpError>> for Res4 {
    fn from(value: Result<StreamingClient, HttpError>) -> Self {
        let client = match value {
            Ok(client) => client,
            Err(err) => return Self::GetClientError(Error::HttpClient(err)),
        };

        match client {
            StreamingClient::RangesSupported(c) => Self::GotRangesSupported(c),
            StreamingClient::RangesRefused(c) => Self::GotRangesRefused(c),
        }
    }
}
