mod queue;
use queue::Queue;

use crate::database::PodcastDb;
use iced::Element;
use iced::widget::Text;

#[derive(Debug)]
pub struct Home {
    db: PodcastDb,
    pub queue: Queue,
}

impl Home {
    pub fn from_db(db: PodcastDb) -> Self {
        Self {
            queue: Default::default(),
            db,
        }
    }
    pub fn view(&mut self) -> Element<crate::Message> {
        Element::new(Text::new("test"))
    }
}

