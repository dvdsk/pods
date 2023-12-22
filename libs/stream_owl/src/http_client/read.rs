use std::collections::VecDeque;
use std::ops::Range;

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::{Body, Incoming};

use crate::target::StreamTarget;

use super::size::Size;
// todo fix error, should be task stream error?
use super::{Client, Error};

#[derive(Debug)]
pub(crate) struct InnerReader {
    stream: Incoming,
    client: Client,
    buffer: VecDeque<u8>,
}

#[derive(Debug)]
pub(crate) enum Reader {
    PartialData {
        inner: InnerReader,
        range: Range<u64>,
    },
    AllData {
        inner: InnerReader,
        total_size: u64,
    },
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
            Reader::PartialData {
                range,
                inner: InnerReader {
                    client, stream: _, ..
                },
            } => client,
            Reader::AllData {
                total_size,
                inner: InnerReader { client, .. },
            } => client,
        })
    }

    fn inner(&mut self) -> &mut InnerReader {
        match self {
            Reader::PartialData { inner, .. } => inner,
            Reader::AllData { inner, .. } => inner,
        }
    }

    pub(crate) fn stream_size(&self) -> Size {
        match self {
            Reader::PartialData { inner, .. } => inner.client.size.clone(),
            Reader::AllData { inner, .. } => inner.client.size.clone(),
        }
    }

    /// async cancel safe, any bytes read will be written or bufferd by the reader
    /// if you want to track the number of bytes written use a wrapper around the writer
    #[tracing::instrument(level = "trace", skip(target, self))]
    pub(crate) async fn stream_to_writer(
        &mut self,
        target: &StreamTarget,
        max: Option<usize>,
    ) -> Result<(), Error> {
        if let Reader::PartialData { range, .. } = self {
            target.set_pos(range.start)
        }
        self.inner().stream_to_writer(target, max).await
    }
}

impl InnerReader {
    pub(crate) fn new(stream: Incoming, client: Client) -> Self {
        Self {
            stream,
            client,
            buffer: VecDeque::new(),
        }
    }

    /// async cancel safe, any bytes read will be written or bufferd by the reader
    /// if you want to track the number of bytes written use a wrapper around the writer
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) async fn stream_to_writer(
        &mut self,
        output: &StreamTarget,
        max: Option<usize>,
    ) -> Result<(), Error> {
        let max = max.unwrap_or(usize::MAX);
        let mut n_read = 0usize;

        if !self.buffer.is_empty() {
            n_read += self.write_from_buffer(max, output).await?;
        }

        while n_read < max {
            // cancel safe: not a problem if a frame is lost as long as
            // we do not mark them as written
            let Some(data) = get_next_data_frame(&mut self.stream).await? else {
                return Ok(());
            };

            // n_read is never larger then max
            let split = data.len().min(max - n_read);
            let (to_write, to_store) = data.split_at(split);

            output.append(to_write).await.map_err(Error::WritingData)?;
            n_read += to_write.len();
            self.buffer.extend(to_store);
        }

        Ok(())
    }

    // Is cancel safe, no bytes will be removed from buffer before
    // they are written. Returns number of bytes written
    #[tracing::instrument(level = "trace", skip(output, self), ret)]
    async fn write_from_buffer(
        &mut self,
        max: usize,
        output: &StreamTarget,
    ) -> Result<usize, Error> {
        let to_take = self.buffer.len().min(max);
        let from_buffer: Vec<_> = self.buffer.range(0..to_take).copied().collect();
        let mut to_write = from_buffer.as_slice();
        let mut total_written = 0;
        Ok(loop {
            let just_written = output
                .append(&from_buffer)
                .await
                .map_err(Error::WritingData)?;
            // remove only what we wrote to prevent losing data on cancel
            self.buffer.drain(0..just_written);
            to_write = &to_write[just_written..];

            total_written += just_written;
            if to_write.is_empty() {
                break total_written;
            }
        })
    }
}

#[tracing::instrument(level = "debug", err)]
async fn get_next_data_frame(stream: &mut Incoming) -> Result<Option<Bytes>, Error> {
    loop {
        let Some(frame) = stream.frame().await else {
            tracing::trace!("no more data frames");
            return Ok(None);
        };
        let frame = frame.map_err(Error::ReadingBody)?;

        if let Ok(data) = frame.into_data() {
            return Ok(Some(data));
        }
    }
}
