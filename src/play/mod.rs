// use futures::Stream;
use std::io::BufReader;
use std::time::Instant;
use std::sync::mpsc;
use iced::{Command, button, Column, Text, Row, Space, Button, Length};
use crate::Message;
use crate::database;
use crate::download::FileType;

mod stream;
pub use stream::ReadableReciever;
pub mod subscribe;

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
    pub fn info(&self) -> Option<&TrackInfo> {
        match self {
            Track::Stream(info, ..) => Some(info),
            Track::File(info, ..) => Some(info),
            Track::None => None,
        }
    }
    /// Duration in seconds
    pub fn duration(&self) -> f32 {
        self.info()
            .map(|i| i.duration)
            .unwrap_or(0f32)
    }
}

pub struct TrackInfo {
    pub id: database::EpisodeKey,
    pub title: String,
    pub paused: bool,
    pub duration: f32,
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

    pub sink: Option<rodio::Sink>,
    pub output_stream: Option<(rodio::OutputStream, rodio::OutputStreamHandle)>,

    db: database::PodcastDb,
    pub rx: Option<Arc<Mutex<mpsc::Receiver<bytes::Bytes>>>>,

    last_started: Option<Instant>,
    last_stored: Option<f32>,
    offset: f32,
}

impl Player {
    pub fn from_db(db: database::PodcastDb) -> Self {
        Self {
            controls: Controls { skip_dur: 5f32, .. Controls::default()},
            current: Track::None,
            sink: None,
            output_stream: None,
            db,
            rx: None,
            last_started: None,
            last_stored: None,
            offset: 0f32,
        }
    }

    pub fn start_stream(&mut self) {
        let rx = self.rx.take().unwrap();
        let rx = Arc::try_unwrap(rx).unwrap();
        let rx = rx.into_inner().unwrap();
        let rrx = ReadableReciever::new(rx);
        let source = rodio::Decoder::new_mp3(rrx).unwrap();
        self.start_play(source);
    }

    fn start_play<S>(&mut self, source: S)
        where
            S: rodio::source::Source + Send + 'static,
            S: rodio::source::SourceExt + Send + 'static,
            S::Item: rodio::Sample,
            S::Item: Send,
    {
        let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        sink.append_seekable(source);

        self.sink = Some(sink);
        self.output_stream = Some((stream, stream_handle));
        self.last_started = Some(std::time::Instant::now());
        self.offset = 0f32;
    }

    fn pos(&self) -> f32 {
        let elapsed = self.last_started
            .map(|t| t.elapsed().as_secs_f32() )
            .unwrap_or(0f32);
        self.offset+elapsed
    }

    pub fn should_store_pos(&mut self) -> Option<f32> {
        let pos = self.pos();
        if pos > self.last_stored.unwrap_or(0f32) + 5f32 {
            self.last_stored = Some(pos);
            Some(pos)
        } else {
            None
        }
    }

    fn stop(&mut self) {
        self.sink.take();
    }

    pub fn add_stream(&mut self, id: database::EpisodeKey) {
        self.stop();

        let meta = self.db.get_episode_ext(id).unwrap();
        self.current = Track::Stream(
            TrackInfo {
                id,
                title: String::default(),
                paused: false,
                duration: dbg!(meta.duration),
            }, 
            0f32, 
            meta.stream_url);
    }

    // TODO figure out better way to get extension into here
    pub fn add_file(&mut self, id: database::EpisodeKey, file_type: FileType, starting_pos: f32) {
        use crate::download::base_file_path;
        self.stop();

        let episode = self.db.get_episode_ext(id).unwrap();
        let mut path = base_file_path(&episode);
        path.set_extension(file_type.as_str());

        let file = std::fs::File::open(&path).unwrap();
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
        self.start_play(source);
        self.sink.as_mut().unwrap().set_pos(starting_pos);
        self.offset = starting_pos;

        self.current = Track::File(
            TrackInfo {
                id,
                title: String::default(),
                paused: false,
                duration: episode.duration,
            }, 
            path);
    }

    pub fn stream_ready(&self, p: f32) -> bool {
        const MINIMUM_BUF_DUR: f32 = 60f32; // duration (seconds) that needs to be downloaded before we start playing
        let downloaded_duration = p/100f32*self.current.duration();
        let sink_empty = self.sink.as_ref().map(|s| s.empty()).unwrap_or(true); 
        let downloaded_enough = downloaded_duration > MINIMUM_BUF_DUR;

        sink_empty && downloaded_enough
    }

    pub fn skip(&mut self, dur: f32) {
        let pos = self.pos();
        let target = f32::max(pos+dur, 0f32);
        let target = match &self.current {
            Track::None => return,
            // can not seek further then what was downloaded
            // because of varying compression throughout the stream we
            // keep a safety bound of 10 percent. TODO FIXME make sure 
            // the visualisation does not show beyond the safety bound
            Track::Stream(info, dl_pos_percent, _) => {
                if *dl_pos_percent <= 100. {
                    let dl_pos_secs = dl_pos_percent*(info.duration)/100.;
                    let dl_pos_secs = dl_pos_secs * 0.9;
                    f32::min(target, dl_pos_secs)
                } else {
                    target
                }
            }
            // can not seek beyond the length of the audio file
            Track::File(info, _) => f32::min(target, info.duration),
        };
        self.offset += target-pos;
        self.sink.as_mut().unwrap().set_pos(target);
    }

    pub fn play_pause(&mut self) -> Command<crate::Message> {
        if let Some(elapsed) = self.last_started
            .take()
            .map(|t| t.elapsed()) {
            self.offset += elapsed.as_secs_f32();
            self.sink.as_mut().unwrap().pause();
        } else {
            self.last_started = Some(Instant::now());
            self.sink.as_mut().unwrap().play();
        }
        Command::none()
    }

    pub fn view(&mut self) -> Column<Message> {
        let column = Column::new();
        match &self.current {
            Track::None => column,
            Track::Stream(info, download, _) => {
                let download_progress_bar = iced::ProgressBar::new(0.0..=100.0, *download);
                let playback_bar = iced::ProgressBar::new(0.0..=info.duration, self.pos());
                let controls = Self::view_controls(&mut self.controls, info);
                column.push(download_progress_bar).push(playback_bar).push(controls)
            }
            Track::File(info, _) => {
                let playback_bar = iced::ProgressBar::new(0.0..=info.duration, self.pos());
                let controls = Self::view_controls(&mut self.controls, info);
                column.push(playback_bar).push(controls)
            }
        }
    }

    fn view_controls<'a>(controls: &'a mut Controls, status: &'a TrackInfo) -> Row<'a, Message> {
        let (button_text, button_action) = if status.paused {
            (Text::new("Pause"), Message::PlayPause)
        } else {
            (Text::new("Resume"), Message::PlayPause)
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
