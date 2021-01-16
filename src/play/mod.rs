// use futures::Stream;
use std::sync::mpsc;
use bytes::Bytes;
use rodio::Decoder;
use eyre::WrapErr;
use iced::{button, Column, Text, Row, Space, Button, Length};
use crate::Message;
use crate::database;

mod stream;
pub use stream::ReadableReciever;
pub mod subscribe;

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

type Url = String;
type StreamPos = f32;
pub enum Track {
    Stream(TrackInfo, StreamPos, Url),
    File(TrackInfo, std::path::PathBuf),
    None,
}

impl Track {
    pub fn set_streampos(&mut self, new_pos: StreamPos) {
        if let Track::Stream(_, pos, _) = self {
            *pos = new_pos;
        } else {
            panic!("Track variant is not Stream")
        }
    }
}

#[derive(Default)]
pub struct TrackInfo {
    pub title: String,
    pub paused: bool,
    pub pos: f32,
    pub length: f32,
}

#[derive(Default)]
struct Controls {
    play_pauze: button::State,
    skip_forward: button::State,
    skip_backward: button::State,
    skip_dur: f32,
}

use std::sync::{Arc, Mutex};
pub struct Player {
    controls: Controls,
    pub current: Track,
    pub sink: rodio::Sink,
    output_stream: rodio::OutputStream,
    db: database::Episodes,
    pub rx: Option<Arc<Mutex<mpsc::Receiver<bytes::Bytes>>>>,
}

impl Player {
    pub fn from_db(db: &database::Episodes) -> Self {
        let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        Self {
            controls: Controls::default(),
            current: Track::None,
            sink,
            output_stream: stream,
            db: db.clone(),
            rx: None,
        }
    }

    pub fn play_stream(&mut self, key: database::episodes::Key) {
        let meta = self.db.get(key).unwrap();
        let url = meta.stream_url;
        self.current = Track::Stream(
            TrackInfo {
                title: String::default(),
                paused: false,
                pos: 0.0,
                length: 0.0,
            }, 0f32, url);
    }

    pub fn skip(&mut self, dur: f32) {
        self.sink.set_pos(dur);
    }

    pub fn view(&mut self) -> Column<Message> {
        let mut column = Column::new();
        match &self.current {
            Track::None => column,
            Track::Stream(info, pos, url) => {
                let stream_progress_bar = iced::ProgressBar::new(0.0..=100.0, *pos);
                let controls = Self::view_controls(&mut self.controls, info);
                column.push(stream_progress_bar).push(controls)
            }
            Track::File(info, path) => {
                let controls = Self::view_controls(&mut self.controls, info);
                column.push(controls)
            }
        }
    }

    fn view_controls<'a>(controls: &'a mut Controls, status: &'a TrackInfo) -> Row<'a, Message> {
        let (button_text, button_action) = if status.paused {
            (Text::new("Pause"), Message::Pauze)
        } else {
            (Text::new("Resume"), Message::Resume)
        };

        let Controls {play_pauze, skip_forward, skip_backward, skip_dur} = controls;
        Row::new()
            .push(Space::with_width(Length::FillPortion(2)))
            .push(Button::new(play_pauze, button_text)
                .on_press(button_action)
                .width(Length::FillPortion(1)))
            .push(Button::new(skip_forward, Text::new("fwd"))
                .on_press(Message::Skip(*skip_dur))
                .width(Length::FillPortion(1)))
            .push(Button::new(skip_backward, Text::new("bck"))
                .on_press(Message::Skip(-1f32*(*skip_dur)))
                .width(Length::FillPortion(1)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{Read, Seek, SeekFrom};

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
}
