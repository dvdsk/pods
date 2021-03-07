use std::collections::HashMap;

use iced::{button, Button, Align, Length, Space, Element, Text, HorizontalAlignment, Row, Column, Container};
use iced_native::{Widget, layout, Hasher, mouse, Size, Layout, Point, Rectangle, Clipboard};
use iced_native::event::{Event, Status};
use iced_native::text::Renderer as _;
use iced_graphics::{Vector, backend, Backend, Defaults, Primitive, Renderer, Color, Background, Font};

use crate::database::{Episode, Progress, Date};
use crate::download::FileType;
use crate::Message;
// use super::style;

const TITLE_SIZE: f32 = 70.0;
const META_SIZE: f32 = 20.0;
const WIDTH: f32 = 5.0;

struct ElementsLayout {
    bounds: Rectangle,
    title_bounds: Rectangle,
    pub_bounds: Rectangle,
    h_line: (f32,f32,f32),
    v_line: (f32,f32,f32),
    plus: (f32,f32),
}

impl Collapsed {
    fn layout<B: backend::Text + Backend>(&self, layout: Layout, renderer: &mut Renderer<B>) -> ElementsLayout {
        // TODO title bounds
        let Rectangle {x, y, width, height} = layout.bounds();
        dbg!(y);

        let title_bounds = Size::new(0.8*width, height);
        let (w, h) = renderer.measure(&self.title, TITLE_SIZE as u16, Font::Default, title_bounds);
        dbg!(h);
        let title_bounds = Rectangle {x, y, 
            width: w, 
            height: h};
        let pub_bounds = Rectangle {x, 
            y: y+title_bounds.height, 
            width: 0.4*width, 
            height: 2.0*META_SIZE};

        let h_line = (0.0, x+width, height-WIDTH);
        let v_line = (title_bounds.y, pub_bounds.y+pub_bounds.height, x+0.8*width);
        let plus = (0.9*width, y + TITLE_SIZE);

        ElementsLayout {
            bounds: layout.bounds(),
            title_bounds,
            pub_bounds,
            h_line,
            v_line,
            plus,
        }
    }
}

#[derive(Debug)]
pub struct Collapsed {
    pub title: String,
    pub published: String,
    pub duration: String,
    pub title_bounds: Size,
    pub widget_bounds: Size,
}


impl Collapsed {
    pub fn new(title: String, age: String, duration: String) -> Self {
        Self {
            title: String::from("Test long title of a random podcast, look it is long"),
            published: String::from("5 weeks ago"),
            duration: String::from("22:30"),
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
        let Size {width, height} = limits.max();
        let text_bounds = Size::new(0.8*width, height);
        let (_, mut height) = renderer.measure(&self.title, TITLE_SIZE as u16, Font::Default, text_bounds);
        height += 2.0*META_SIZE;
        height += WIDTH;

        layout::Node::new(Size::new(width, height))
    }
    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash;
        self.title.hash(state);
        // state
    }
    fn draw(&self, 
        renderer: &mut Renderer<B>, 
        _defaults: &Defaults, 
        layout: Layout<'_>, 
        _cursor_position: Point, 
        _viewport: &Rectangle
    ) -> (Primitive, mouse::Interaction) {
        // TODO meta bounds
        let layout = self.layout(layout, renderer);
        let primitives = self.primitives(layout);

        (primitives, mouse_grabbed())
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

impl Collapsed {
    // https://docs.rs/iced_graphics/0.1.0/iced_graphics/enum.Primitive.html
    fn primitives(&self, layout: ElementsLayout) -> Primitive {
        use Primitive::*;

        let (x1,x2,y) = layout.h_line;
        let h_line = h_line(x1, x2, dbg!(y), WIDTH, Color::BLACK);
        let (y1,y2,x) = layout.v_line;
        let v_line = v_line(y1, y2, x, WIDTH, Color::BLACK);
        let mesh = merge_mesh2d(h_line, v_line);
        let (x,y) = layout.plus;
        let plus = plus(x, y, TITLE_SIZE, WIDTH, Color::BLACK);
        let mesh = merge_mesh2d(mesh, plus);

        let mesh = Primitive::Mesh2D{buffers: mesh, size: layout.bounds.size()};

        let title = text_left_aligned(
            self.title.clone(), 
            dbg!(layout.title_bounds),
            TITLE_SIZE);

        let published = text_left_aligned(
            self.published.clone(), 
            dbg!(layout.pub_bounds),
            META_SIZE);
        
        let primitives = vec![mesh, title, published];
        let group = Group{primitives};
        let mesh = Primitive::Translate {
            translation: Vector::new(layout.bounds.x, layout.bounds.y),
            content: Box::new(group)
        };
        mesh
    }
}

/// draws a plus centred at x,y
fn plus(x: f32, y: f32, size: f32, stroke: f32, color: Color) -> Mesh2D {
    let r = size/2.0;
    let h_line = h_line(x-r, x+r, y-stroke/2.0, stroke, color);
    let v_line = v_line(y-r, y+r, x-stroke/2.0, stroke, color);
    merge_mesh2d(h_line, v_line)
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

fn text_left_aligned(text: String, mut bounds: Rectangle, size: f32) -> Primitive {
    Primitive::Text {
        content: text,
        bounds,
        color: Color::BLACK,
        size,
        font: Font::Default,
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: iced::VerticalAlignment::Top,
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
