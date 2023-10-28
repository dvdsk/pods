use std::collections::VecDeque;

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::{Client, Error, InnerClient};

pub(crate) struct InnerReader {
    stream: Incoming,
    client: InnerClient,
    buffer: VecDeque<u8>,
}

pub(crate) enum Reader {
    PartialData(InnerReader),
    AllData(InnerReader),
}

impl Reader {
    pub(crate) fn into_client(self) -> Client {
        match self {
            Reader::PartialData(InnerReader { client, .. }) => Client {
                should_support_range: true,
                inner: client,
            },
            Reader::AllData(InnerReader { client, .. }) => Client {
                should_support_range: false,
                inner: client,
            },
        }
    }

    fn inner(&mut self) -> &mut InnerReader {
        match self {
            Reader::PartialData(inner) => inner,
            Reader::AllData(inner) => inner,
        }
    }

    pub(crate) async fn read(
        &mut self,
        buffer: impl AsyncWrite + Unpin,
        max: Option<usize>,
    ) -> Result<(), Error> {
        self.inner().read(buffer, max).await
    }
}

impl InnerReader {
    pub(crate) fn new(stream: Incoming, client: InnerClient) -> Self {
        Self {
            stream,
            client,
            buffer: VecDeque::new(),
        }
    }

    pub(crate) async fn read(
        &mut self,
        mut output: impl AsyncWrite + Unpin,
        max: Option<usize>,
    ) -> Result<(), Error> {
        let max = max.unwrap_or(usize::MAX);
        let mut n_read = 0usize;

        let to_take = self.buffer.len().min(max);
        let from_buffer: Vec<_> = self.buffer.drain(0..to_take).collect();
        output
            .write(&from_buffer)
            .await
            .map_err(Error::WritingData)?;

        while n_read < max {
            let Some(data) = get_next_data_frame(&mut self.stream).await? else {
                return Ok(());
            };

            // n_read is never larger then max
            let split = data.len().min(max - n_read);
            let (to_write, to_store) = data.split_at(split);

            n_read += to_write.len();
            output
                .write_all(to_write)
                .await
                .map_err(Error::WritingData)?;
            self.buffer.extend(to_store);
        }

        Ok(())
    }
}

async fn get_next_data_frame(stream: &mut Incoming) -> Result<Option<Bytes>, Error> {
    loop {
        let Some(frame) = stream.frame().await else {
            return Ok(None);
        };
        let frame = frame.map_err(Error::ReadingBody)?;

        if let Ok(data) = frame.into_data() {
            return Ok(Some(data));
        }
    }
}
