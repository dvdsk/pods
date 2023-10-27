use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::{Client, Error, InnerClient};

pub(crate) enum Reader {
    PartialData {
        stream: Incoming,
        inner: InnerClient,
    },
    AllData {
        stream: Incoming,
        inner: InnerClient,
    },
}

impl Reader {
    pub(crate) fn into_client(self) -> Client {
        match self {
            Reader::PartialData { stream, inner } => Client {
                should_support_range: true,
                inner,
            },
            Reader::AllData { stream, inner } => Client {
                should_support_range: false,
                inner,
            },
        }
    }

    fn mut_stream(&mut self) -> &mut Incoming {
        match self {
            Reader::PartialData { stream, .. } => stream,
            Reader::AllData { stream, .. } => stream,
        }
    }

    pub(crate) async fn read(
        &mut self,
        mut buffer: impl AsyncWrite + Unpin,
        max: Option<usize>,
    ) -> Result<(), Error> {
        let mut stream = self.mut_stream();
        let max = max.unwrap_or(usize::MAX);
        let mut n_read = 0usize;

        while n_read < max {
            let Some(data) = get_next_data_frame(&mut stream).await.transpose()? else {
                return Ok(());
            };

            if data.len() > max {
                todo!()
            }
            n_read += data.len();
            buffer.write_all(&data);
        }

        Ok(())
    }
}

async fn get_next_data_frame(stream: &mut Incoming) -> Option<Result<Bytes, Error>> {
    loop {
        let frame = match stream.frame().await? {
            Ok(frame) => frame,
            Err(e) => return Some(Err(Error::ReadingBody(e))),
        };

        if let Ok(data) = frame.into_data() {
            return Some(Ok(data));
        }
    }
}
