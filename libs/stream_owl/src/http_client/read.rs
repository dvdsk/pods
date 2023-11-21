use std::collections::VecDeque;

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::{Body, Incoming};

use super::size::Size;
// todo fix error, should be task stream error?
use super::{Client, Error, InnerClient};
use crate::Appender;

#[derive(Debug)]
pub(crate) struct InnerReader {
    stream: Incoming,
    client: InnerClient,
    buffer: VecDeque<u8>,
    size_hint: Size,
}

#[derive(Debug)]
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
                stream: _,
                size_hint,
                ..
            }) => Client {
                should_support_range: true,
                size: size_hint,
                inner: client,
            },
            Reader::AllData(InnerReader {
                client, size_hint, ..
            }) => Client {
                should_support_range: false,
                size: size_hint,
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

    /// async cancel safe, any bytes read will be written or bufferd by the reader
    /// if you want to track the number of bytes written use a wrapper around the writer
    #[tracing::instrument(level = "trace", skip(appender, self), ret)]
    pub(crate) async fn read_to_writer(
        &mut self,
        mut appender: impl Appender,
        max: Option<usize>,
    ) -> Result<(), Error> {
        self.inner().read_to_writer(&mut appender, max).await
    }
}

impl InnerReader {
    pub(crate) fn new(stream: Incoming, client: InnerClient, size_hint: Size) -> Self {
        Self {
            stream,
            client,
            buffer: VecDeque::new(),
            size_hint,
        }
    }

    /// async cancel safe, any bytes read will be written or bufferd by the reader
    /// if you want to track the number of bytes written use a wrapper around the writer
    #[tracing::instrument(level = "trace", skip_all, ret)]
    pub(crate) async fn read_to_writer(
        &mut self,
        output: &mut impl Appender,
        max: Option<usize>,
    ) -> Result<(), Error> {
        let max = max.unwrap_or(usize::MAX);
        let mut n_read = 0usize;

        // if !self.buffer.is_empty() {
            n_read += self.write_from_buffer(max, output).await?;
        // }

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
        output: &mut impl Appender,
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

#[tracing::instrument(level="debug", err)]
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
