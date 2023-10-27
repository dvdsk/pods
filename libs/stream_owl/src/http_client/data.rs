use bytes::Bytes;
use hyper::body::{Incoming, Body};

use super::Error;

pub(crate) enum Data {
    FirstRange {
        bytes: Vec<Bytes>,
    },
    Streaming {
        first_chunk: Option<Bytes>,
        stream: Incoming,
    },
}

async fn get_next_data_frame(stream: &mut Incoming) -> Result<Bytes, Error> {
    // while
    // let frame = stream.frame().await.ok_or(Error::MissingFrame)??;
    todo!()
}

impl std::fmt::Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Data::FirstRange { bytes } => f
                .debug_struct("Data::FirstRange")
                .field("bytes", &bytes.len())
                .finish(),
            Data::Streaming { first_chunk, .. } => f
                .debug_struct("Data::Streaming")
                .field("first_chunk", &first_chunk.as_ref().map(|b| b.len()))
                .field("stream", &"*")
                .finish(),
        }
    }
}

impl Data {
    pub(crate) async fn from_body(mut stream: Incoming, requested_size: usize) -> Result<Data, Error> {
        // a http frame has a max size of ~16.7MB
        if let Some(upper_bound) = stream.size_hint().upper() {
            if upper_bound as usize > requested_size {
                return Ok(Self::Streaming {
                    first_chunk: None,
                    stream,
                });
            }
        }

        let mut bytes = Vec::new();
        let mut collected = 0;
        while collected < requested_size {
            let data = get_next_data_frame(&mut stream).await?;
            if data.len() > requested_size {
                return Ok(Self::Streaming {
                    first_chunk: Some(data),
                    stream,
                });
            }
            collected += data.len();
            bytes.push(data);
        }
        Ok(Self::FirstRange { bytes })
    }
}
