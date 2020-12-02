// use futures::Stream;
use std::sync::mpsc;
use bytes::Bytes;
use std::io::{self, Seek, SeekFrom, Read, ErrorKind};
use rodio::Decoder;
use eyre::WrapErr;

/* design:
 * implement a readable storage that grows via appending trough an 
 * mpsc. Then use a separately running function to 'feed' that mpsc
 * from a http stream. This storage then is the basis for a 
 * rodio::decoder::Decoder from which we build a rodio::Source. That
 * is the played using rodio.
 */

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

impl Read for ReadableReciever {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let needed = buf.len();
        let mut unread_buffer = self.buffer.len() - self.offset;
        if needed <= unread_buffer {
            // fill buf from buffer
            buf.clone_from_slice(&self.buffer[self.offset..self.offset+needed]);
            return Ok(needed);
        }

        // get extra bytes, and put them in the buffer
        // if no bytes are gotten this or the next call to read
        // will return 0 indicating end of file
        let bytes = self.rx.recv().unwrap_or(bytes::Bytes::new());
        self.buffer.extend_from_slice(&bytes);
        unread_buffer += bytes.len();
        
        let read = if needed <= unread_buffer {
            // got what we needed
            buf.clone_from_slice(&self.buffer[self.offset..self.offset+needed]);
            needed
        } else {
            // less bytes then needed, return what we got do not block
            buf[..unread_buffer].clone_from_slice(&self.buffer[self.offset..self.offset+unread_buffer]);
            unread_buffer
        };
        self.offset += read;
        Ok(read)
    }
}

impl std::io::Seek for ReadableReciever {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        use SeekFrom::*;
        match pos {
            Current(p) if self.offset as i64 +p > self.buffer.len() as i64 => 
                Err(io::Error::new(ErrorKind::UnexpectedEof, "cannot seek after end of reader")),
            Current(p) if self.offset as i64 +p < 0 => 
                Err(io::Error::new(ErrorKind::UnexpectedEof, "cannot seek before start of reader")),
            Start(p) if p > self.buffer.len() as u64 =>
                Err(io::Error::new(ErrorKind::UnexpectedEof, "cannot seek after end of reader")),
            End(p) if self.buffer.len() as i64 + p < 0 =>
                Err(io::Error::new(ErrorKind::UnexpectedEof, "cannot seek before start of reader")),
            End(p) if p > 0 =>
                Err(io::Error::new(ErrorKind::UnexpectedEof, "cannot seek after end of reader")),

            Start(p) => {self.offset = p as usize; Ok(self.offset as u64)}
            Current(p) => {self.offset = (self.offset as i64 + p) as usize; Ok(self.offset as u64)}
            End(p) => {self.offset = (self.offset as i64 + p) as usize; Ok(self.offset as u64)}
        }
    }
}

#[test]
fn test_readable_reciever_seek_read_exact() {
    let (tx, rx) = mpsc::channel();
    let mut readable_rx = ReadableReciever::new(rx);
    const T1: &str = "Hello world!";
    const T2: &str = " are you getting this";

    let t1 = Bytes::from(T1);
    let t2 = Bytes::from(T2);
    tx.send(t1).unwrap();

    let mut buffer = vec![0;T1.len()];
    readable_rx.read_exact(&mut buffer).unwrap();
    assert_eq!(T1.as_bytes(), buffer);

    tx.send(t2).unwrap();
    readable_rx.seek(SeekFrom::Start(0)).unwrap();
    let mut buffer = vec![0;T1.len()+T2.len()];
    readable_rx.read_exact(&mut buffer).unwrap();
    assert_eq!([T1,T2].concat().as_bytes(), buffer);
}

#[test]
fn test_readable_reciever_seek_read_string_eof() {
    use std::thread;

    let (tx, rx) = mpsc::channel();
    let mut readable_rx = ReadableReciever::new(rx);
    const T1: &str = "Hello world!";
    const T2: &str = " are you getting this";

    let t1 = Bytes::from(T1);
    tx.send(t1).unwrap();

    let child = thread::spawn(move || {
        let mut buffer = String::new();
        readable_rx.read_to_string(&mut buffer).unwrap();
        assert_eq!([T1,T2].concat(), buffer);
    });

    let t2 = Bytes::from(T2);
    tx.send(t2).unwrap();

    drop(tx); // indicates the end (EOF)
    // only now the child thread can read to end of file
    child.join().unwrap();
}

struct WebToDecoderStream {
    res: reqwest::Response,
    tx: mpsc::Sender<Bytes>,
}

// TODO support more then mp3 if needed [are podcasts always mp3?]
async fn start_streaming(url: &str) -> eyre::Result<(Decoder<ReadableReciever>, WebToDecoderStream)> {
    let (tx, rx) = mpsc::channel();
    let readable_rx = ReadableReciever::new(rx);
    dbg!();
    let mut recieved = 0;
    let mut res = reqwest::get(url).await?;
    while recieved < 32_000 {
        // get some data into readable_rx. Otherwise  creating the 
        // decoder will fail as it has no header data 
        if let Some(chunk) = res.chunk().await.unwrap() {
            dbg!();
            recieved += chunk.len();
            tx.send(chunk).unwrap();
        }
    }
    dbg!();
    let decoder = Decoder::new_mp3(readable_rx).unwrap();
    dbg!();
    Ok((decoder, WebToDecoderStream{res, tx}))
}

async fn continue_streaming(stream: WebToDecoderStream) -> eyre::Result<()> {
    let WebToDecoderStream {mut res, tx} = stream;
    while let Some(chunk) = res.chunk().await.wrap_err("stream failed")? {
        dbg!(chunk.len());
        tx.send(chunk).unwrap();
    }
    Ok(())
}

#[test]
fn stream_mp3() {
    use tokio::runtime::Runtime;
    const URL: &str = "http://icecast.omroep.nl/radio2-bb-mp3";

    // Create the runtime
    Runtime::new()
        .unwrap()
        .block_on(async {
            let (source, passer) = start_streaming(URL).await.unwrap();
            let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
            sink.append(source);
            continue_streaming(passer).await.unwrap();
        });

}
