// use futures::Stream;
use std::sync::mpsc;
use bytes::Bytes;
use rodio::Decoder;
use eyre::WrapErr;
use iced::{button, Column, Text, Row, Space, Button, Length};
use crate::Message;
use crate::database;

mod stream;
use stream::ReadableReciever;

/* design:
 * implement a readable storage that grows via appending trough an 
 * mpsc. Then use a separately running function to 'feed' that mpsc
 * from a http stream. This storage then is the basis for a 
 * rodio::decoder::Decoder from which we build a rodio::Source. That
 * is the played using rodio.
 */
pub struct WebToDecoderStream {
    res: reqwest::Response,
    tx: mpsc::Sender<Bytes>,
}

// TODO support more then mp3 if needed [are podcasts always mp3?]
async fn start_streaming(url: &str) -> eyre::Result<(Decoder<ReadableReciever>, WebToDecoderStream)> {
    let (tx, rx) = mpsc::channel();
    let readable_rx = ReadableReciever::new(rx);

    let mut recieved = 0;
    let mut res = reqwest::get(url).await?;
    while recieved < 32_000 {
        // get some data into readable_rx. Otherwise  creating the 
        // decoder will fail as it has no header data 
        if let Some(chunk) = res.chunk().await.unwrap() {
            recieved += chunk.len();
            tx.send(chunk).unwrap();
        }
    }
    let decoder = Decoder::new_mp3(readable_rx).unwrap();
    Ok((decoder, WebToDecoderStream{res, tx}))
}

pub async fn continue_streaming(stream: WebToDecoderStream) -> eyre::Result<()> {
    let WebToDecoderStream {mut res, tx} = stream;
    while let Some(chunk) = res.chunk().await.wrap_err("stream failed")? {
        tx.send(chunk).unwrap();
    }
    Ok(())
}

pub struct Status {
    pub title: String,
    pub paused: bool,
    pub pos: f32,
    pub length: f32,
}

pub struct PlayBack {
    pub status: Option<Status>,
    pub playpauze: button::State,
    sink: rodio::Sink,
    db: database::Episodes,
}

impl PlayBack {
    pub fn from_db(db: &database::Episodes) -> Self {
        let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        Self {
            status: None,
            playpauze: button::State::new(),
            sink,
            db: db.clone(),
        }
    }
}

impl PlayBack {
    pub async fn play(&self, key: database::episodes::Key) {//-> Command<PlayProgress> WebToDecoderStream  {
        let meta = self.db.get(key).unwrap();
        let url = meta.stream_url;
        let (source, passer) = start_streaming(&url).await.unwrap();
        self.sink.append(source);
        // Message::P passer
    }

    pub fn view(&mut self) -> Column<Message> {
        let column = Column::new();
        if self.status.is_none() {return column;}
        let status = self.status.as_ref().unwrap();

        let (button_text, button_action) = if status.paused {
            (Text::new("Pause"), Message::Pauze)
        } else {
            (Text::new("Resume"), Message::Resume)
        };

        let progress_bar = iced::ProgressBar::new(0.0..=status.length, status.pos);
        let controls = Row::new()
            .push(Space::with_width(Length::FillPortion(2)))
            .push(Button::new(&mut self.playpauze, button_text)
                .on_press(button_action)
                .width(Length::FillPortion(1)));
        
        column.push(progress_bar).push(controls)
    }
}

#[test] // should cause sound output unless the dutch radio stream is down
#[ignore] // run with cargo test mp3 -- --ignored
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
            drop(stream);
        });

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
