// use futures::Stream;
use std::sync::mpsc;
use reqwest::RequestBuilder;
use rodio::{Source, Sample};
use minimp3::{Decoder, Frame};
use std::io::{self, SeekFrom};

/* design:
 * implement a readable storage that grows via appending trough an 
 * mpsc. Then use a separately running function to 'feed' that mpsc
 * from a http stream. This storage then is the basis for a 
 * rodio::decoder::Decoder from which we build a rodio::Source. That
 * is the played using rodio.
 */

// decode and provide 
// stream for rodio
struct Mp3Stream {
    rx: mpsc::Receiver<bytes::Bytes>,
    decoder: Decoder<bytes::Bytes>,
    current_frame: Frame,
    current_frame_offset: usize,
}

// impl Source for Mp3Stream {
//     fn current_frame_len(&self) -> Option<usize> {
//         None
//     }
//     fn channels(&self) -> u16 {
//         1
//     }
//     fn sample_rate(&self) -> u32 {
        
//     }
//     fn total_duration(&self) -> Option<std::time::Duration> {
//         None
//     }
// }

// impl Iterator<Item = Sample> for Mp3Stream {
//     fn next(&mut self) -> Option<Self::Item> {
        
//     }

// }

struct ReadableReciever {
    rx: mpsc::Receiver<bytes::Bytes>,
    /// buffer of all received data, used for seeking
    buffer: Vec<u8>,
    offset: usize,
}

impl ReadableReciever {
    fn new(rx: mpsc::Receiver<bytes::Bytes>) -> Self {
        Self {
            rx,
            buffer: Vec::new(),
            offset: 0,
        }
    }
}

impl std::io::Read for ReadableReciever {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let needed = buf.len();
        let mut unread_buffer = self.buffer.len() - self.offset;
        if needed <= unread_buffer {
            // fill buf from buffer
            buf.clone_from_slice(&self.buffer[self.offset..self.offset+needed]);
            return Ok(needed);
        }

        // get extra bytes, and put them in the buffer
        let bytes = self.rx.recv().unwrap();
        self.buffer.extend_from_slice(&bytes);
        unread_buffer += bytes.len();
        
        let read = if needed <= unread_buffer {
            // got what we needed
            buf.clone_from_slice(&self.buffer[self.offset..self.offset+needed]);
            needed
        } else {
            // less bytes then needed, return what we got do not block
            buf[..unread_buffer].clone_from_slice(&self.buffer[self.offset..self.offset+needed]);
            unread_buffer
        };
        self.offset += read;
        Ok(read)
    }
}

impl std::io::Seek for ReadableReciever {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        todo!()        
    }
}

// type HttpStream = reqwest::Response;
// async fn start_streaming(url: &str) -> eyre::Result<(Mp3Stream, HttpStream)> {
//     let (tx, rx) = mpsc::channel();
//     let mut decoder = Decoder::new(data); //TODO replace data with something that implements read
//     let current_frame = decoder.next_frame().unwrap()?;

//     let mut res = reqwest::get(url).await?;
//     while let Some(chunk) = res.chunk().await? {
//         tx.send(chunk);
//     }

//     let stream = Mp3Stream {
//         decoder,
//         current_frame,
//         current_frame_offset = 0,
//     };
// }

// async fn continue_streaming(stream: HttpStream){
//     let (tx, rx) = mpsc::channel();
//     let stream = Mp3Stream {
//         rx,
//     };

//     while let Some(chunk) = stream.chunk().await? {
//         tx.send(chunk);
//     }
// }

// separate thread for streaming decode? (can be done async)
// source will be moved to audiothread? 
// audio playback is already done by a separate thread
// need way to send back errors from the audiothread... can we?
