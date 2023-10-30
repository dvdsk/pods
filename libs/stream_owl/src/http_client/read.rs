use std::collections::VecDeque;

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::{Body, Incoming};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::size_hint::SizeHint;
use super::{Client, Error, InnerClient};

pub(crate) struct InnerReader {
    stream: Incoming,
    client: InnerClient,
    buffer: VecDeque<u8>,
    size_hint: SizeHint,
}

pub(crate) enum Reader {
    PartialData(InnerReader),
    AllData(InnerReader),
}

#[derive(Debug, thiserror::Error)]
#[error("Can not turn reader into a client while the server is still sending data to be read")]
pub struct StreamNotEmpty;

impl Reader {
    pub(crate) fn try_into_client(mut self) -> Result<Client, StreamNotEmpty> {
        if !self.inner().stream.is_end_stream() {
            return Err(StreamNotEmpty);
        }

        Ok(match self {
            Reader::PartialData(InnerReader {
                client,
                stream,
                size_hint,
                ..
            }) => Client {
                should_support_range: true,
                size_hint,
                inner: client,
            },
            Reader::AllData(InnerReader {
                client, size_hint, ..
            }) => Client {
                should_support_range: false,
                size_hint,
                inner: client,
            },
        })
    }

    pub(crate) async fn into_client(mut self) -> Result<Client, Error> {
        while let Some(_frame) = self
            .inner()
            .stream
            .frame()
            .await
            .transpose()
            .map_err(Error::EmptyingBody)?
        {}
        Ok(self.try_into_client().expect("just emptied the stream"))
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
    pub(crate) fn new(stream: Incoming, client: InnerClient, size_hint: SizeHint) -> Self {
        Self {
            stream,
            client,
            buffer: VecDeque::new(),
            size_hint,
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
                dbg!("stream is out of data, ", self.stream.is_end_stream());
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
