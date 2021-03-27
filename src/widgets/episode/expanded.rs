use iced::{Length, Element, HorizontalAlignment};
use iced_native::{Widget, layout, Hasher, mouse, Size, Layout, Point, Rectangle, Clipboard};
use iced_native::event::{Event, Status};
use iced_native::text::Renderer as _;
use iced_graphics::{Vector, backend, Backend, Defaults, Primitive, Renderer, Color, Font};

use super::Collapsed;

#[derive(Debug)]
pub struct Expanded<Message> {
    collapsed: Collapsed<Message>,
    description: String,
    on_stream: Option<Message>,
    on_add: Option<Message>,
    on_remove: Option<Message>,
}

struct ElementsLayout {
    bounds: Rectangle,
    title_bounds: Rectangle,
    pub_bounds: Rectangle,
    dur_bounds: Rectangle,
    h_line: (f32,f32,f32),
    v_line: (f32,f32,f32),
    plus: (f32,f32),
}

impl<Message> Expanded<Message> {
    pub fn from_collapsed(collapsed: Collapsed<Message>, description: String) -> Self {
        Self {
            collapsed,
            description,
            on_stream: None,
            on_add: None,
            on_remove: None
        }
    }
    pub fn on_stream(&mut self, msg: Message) {
        self.on_stream = Some(msg);
    }
    pub fn on_add(&mut self, msg: Message) {
        self.on_add = Some(msg);
    }
    pub fn on_remove(&mut self, msg: Message) {
        self.on_remove = Some(msg);
    }
}
