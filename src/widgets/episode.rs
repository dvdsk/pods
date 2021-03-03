use std::collections::HashMap;

use iced::{button, Button, Align, Length, Space, Element, Text, HorizontalAlignment, Row, Column, Container};
use iced_native::{Widget, layout, Hasher, mouse, Size, Layout, Point, Rectangle, Clipboard};
use iced_native::event::{Event, Status};
use iced_native::text::Renderer as _;
use iced_graphics::{backend, Backend, Defaults, Primitive, Renderer, Color, Background, Font};

use crate::database::{Episode, Progress, Date};
use crate::download::FileType;
use crate::Message;
// use super::style;

#[derive(Debug)]
pub struct Collapsed {
    pub title: String,
    pub age: String,
    pub duration: String,
    pub title_bounds: Size,
    pub widget_bounds: Size,
}

impl Collapsed {
    pub fn new(title: String, age: String, duration: String) -> Self {
        Self {
            title,
            age,
            duration,
            title_bounds: Size::ZERO,
            widget_bounds: Size::ZERO,
        }
    }
}

impl<Message, B> Widget<Message, Renderer<B>> for Collapsed
where
    B: Backend + backend::Text,
{
    fn width(&self) -> Length {
        Length::Fill
    }
    fn height(&self) -> Length {
        Length::Fill
    }
    fn layout(&self, renderer: &Renderer<B>, limits: &layout::Limits) -> layout::Node {
        let Size {width: w_full, height} = limits.max();
        let text_bounds = Size::new(0.8*w_full, height);
        let (_, height) = renderer.measure("test", 70, Font::Default, text_bounds);
        // let title_bounds = Size::new(width, height);
        let widget_bounds = Size::new(w_full, height);

        layout::Node::new(widget_bounds)
    }
    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash;
        // state
    }
    fn draw(&self, 
        renderer: &mut Renderer<B>, 
        _defaults: &Defaults, 
        layout: Layout<'_>, 
        _cursor_position: Point, 
        _viewport: &Rectangle
    ) -> (Primitive, mouse::Interaction) {
        
        // TODO title bounds
        let Rectangle {x, y, width: w_full, height} = layout.bounds();
        let text_bounds = Size::new(0.8*w_full, height);
        let (width, height) = renderer.measure(
            "Test long title of a random podcast, look it is long", 
            70, Font::Default, text_bounds);
        let title_bounds = Rectangle {x, y, width, height};

        
        // TODO meta bounds

        (primitive(layout, title_bounds), mouse_grabbed())
    }
    fn on_event(&mut self, 
        _event: Event, 
        _layout: Layout<'_>, 
        _cursor_position: Point, 
        _messages: &mut Vec<Message>, 
        _renderer: &Renderer<B>, 
        _clipboard: Option<&dyn Clipboard>
    ) -> Status {
        Status::Ignored
    }
}

impl<'a, Message> Into<Element<'a, Message>> for Collapsed {
    fn into(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

fn primitive(layout: Layout<'_>, title_bounds: Rectangle) -> Primitive {
    use Primitive::*;
    // https://docs.rs/iced_graphics/0.1.0/iced_graphics/enum.Primitive.html
    let size = layout.bounds().size();
    let Rectangle {x, y, width, ..} = layout.bounds();

    let bottem_line = h_line(x, x+width, y, 10.0, Color::BLACK);
    let v_line = v_line(y, y+0.2*width, x+0.8*width, 10.0, Color::BLACK);
    let mesh = merge_mesh2d(bottem_line, v_line);
    let mesh = Primitive::Mesh2D{buffers: mesh, size};

    let title = title(
        "Test long title of a random podcast, look it is long".to_owned(), 
        title_bounds);
    
    let primitives = vec![mesh, title];
    Group{primitives}
}

use iced_graphics::triangle::{Vertex2D, Mesh2D};
/// horizontal line from x1 to x2 at height y
fn h_line(x1: f32, x2: f32, y: f32, width: f32, color: Color) -> Mesh2D {
    let color = color.into_linear();
    let top_l = Vertex2D {position: [x1, y], color};
    let top_r = Vertex2D {position: [x2, y], color};
    let bottom_l = Vertex2D {position: [x1, y+width], color};
    let bottom_r = Vertex2D {position: [x2, y+width], color};
    Mesh2D {
        vertices: vec![bottom_l, top_l, bottom_r, top_r],
        indices: vec![0,1,2,1,2,3],
    }
}
/// vertical line from y1 to y2 at position x
fn v_line(y1: f32, y2: f32, x: f32, width: f32, color: Color) -> Mesh2D {
    let color = color.into_linear();
    let top_l = Vertex2D {position: [x, y2], color};
    let top_r = Vertex2D {position: [x+width, y2], color};
    let bottom_l = Vertex2D {position: [x, y1], color};
    let bottom_r = Vertex2D {position: [x+width, y1], color};
    Mesh2D {
        vertices: vec![bottom_l, top_l, bottom_r, top_r],
        indices: vec![0,1,2,1,2,3],
    }
}

fn title(text: String, bounds: Rectangle) -> Primitive {
    Primitive::Text {
        content: text,
        bounds,
        color: Color::BLACK,
        size: 70.0,
        font: Font::Default,
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: iced::VerticalAlignment::Center,
    }
}

fn mouse_grabbed() -> mouse::Interaction {
    mouse::Interaction::default()
}

use std::iter::Extend;
fn merge_mesh2d(mut a: Mesh2D, mut b: Mesh2D) -> Mesh2D {
    let offset = a.vertices.len() as u32;
    a.vertices.append(&mut b.vertices);
    a.indices.extend(b.indices.drain(..).map(|b| b+offset));
    a
}
