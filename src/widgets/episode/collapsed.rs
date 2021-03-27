use iced::{Length, Element};
use iced_native::{Widget, layout, Hasher, mouse, Size, Layout, Point, Rectangle, Clipboard};
use iced_native::event::{Event, Status};
use iced_native::text::Renderer as _;
use iced_graphics::{Vector, backend, Backend, Defaults, Primitive, Renderer, Color, Font};
use super::super::util::{h_line, v_line, text_left_aligned, text_right_aligned, merge_mesh2d, plus};

const TITLE_SIZE: f32 = 50.0;
pub const META_SIZE: f32 = 20.0;
const PLUS_SIZE: f32 = 30.0;
pub const MARGIN: f32 = PLUS_SIZE/2.0;

pub const WIDTH: f32 = 5.0;
const VLINE_WIDTH: f32 = WIDTH*1.5;
const PLUS_WIDTH: f32 = WIDTH*0.8;

const PLUS_H_SPACE: f32 = PLUS_SIZE *2.0;


pub struct ElementsLayout {
    pub bounds: Rectangle,
    title_bounds: Rectangle,
    pub_bounds: Rectangle,
    dur_bounds: Rectangle,
    h_line: (f32,f32,f32),
    v_line: (f32,f32,f32),
    plus: (f32,f32),
}

impl<Message> Collapsed<Message> {
    pub fn layout_elements(&self, layout: Layout) -> ElementsLayout {
        let Rectangle {width, height, ..} = layout.bounds();

        let y = 0.;
        let x = MARGIN;
        let title_bounds = Rectangle {x, y, 
            width: width - PLUS_H_SPACE - MARGIN,
            height};
        let pub_bounds = Rectangle {x, 
            y: y+height-META_SIZE-WIDTH, 
            width: title_bounds.width, 
            height: 1.0*META_SIZE};

        let x = 0.0;
        let h_line = (x, width, y+height-WIDTH);
        let v_line = (y, pub_bounds.y+pub_bounds.height, width-PLUS_H_SPACE);
        let plus = (
            width + 0.5*VLINE_WIDTH - 0.5*PLUS_H_SPACE, 
            0.5*(title_bounds.height + pub_bounds.height));

        let dur_bounds = Rectangle {
            x: v_line.2 - VLINE_WIDTH,
            .. pub_bounds};

        ElementsLayout {
            bounds: layout.bounds(),
            title_bounds,
            pub_bounds,
            dur_bounds,
            h_line,
            v_line,
            plus,
        }
    }
}

#[derive(Debug)]
pub struct Collapsed<Message> {
    pub on_title: Option<Message>,
    pub on_plus: Option<Message>,

    pub title: String,
    pub published: String,
    pub duration: String,
}


impl<Message> Collapsed<Message> {
    pub fn new(_title: String, _age: String, _duration: String) -> Self {
        Self {
            on_title: None,
            on_plus: None,

            title: String::from("Test long title of a random podcast, look it is long"),
            published: String::from("published: 5 weeks ago"),
            duration: String::from("22:30"),
        }
    }
    pub fn on_title(mut self, msg: Message) -> Self {
        self.on_title = Some(msg);
        self
    }
    pub fn on_plus(mut self, msg: Message) -> Self {
        self.on_plus = Some(msg);
        self
    }
}

impl<Message: Clone, B> Widget<Message, Renderer<B>> for Collapsed<Message>
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
        let text_bounds = Size::new(width - PLUS_H_SPACE - MARGIN, height);
        let (_, mut height) = renderer.measure(&self.title, TITLE_SIZE as u16, Font::Default, text_bounds);
        height += 1.2*META_SIZE;
        height += WIDTH;

        layout::Node::new(Size::new(width, height))
    }
    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash;
        self.title.hash(state);
    }
    fn draw(&self, 
        _renderer: &mut Renderer<B>, 
        _defaults: &Defaults, 
        layout: Layout<'_>, 
        cursor_position: Point, 
        _viewport: &Rectangle
    ) -> (Primitive, mouse::Interaction) {
        // TODO meta bounds
        let layout = self.layout_elements(layout);
        let mouse = mouse_grabbed(&layout, cursor_position);

        let primitives = self.primitives(&layout, true);
        let primitives = Primitive::Group{primitives};
        let primitives = Primitive::Translate {
            translation: Vector::new(layout.bounds.x, layout.bounds.y),
            content: Box::new(primitives)
        };

        (primitives, mouse)
    }
    fn on_event(&mut self, 
        event: Event, 
        layout: Layout<'_>, 
        cursor_position: Point, 
        messages: &mut Vec<Message>, 
        _renderer: &Renderer<B>, 
        _clipboard: Option<&dyn Clipboard>
    ) -> Status {
        use mouse::Event::ButtonReleased;
        use iced_native::touch::Event::FingerPressed;

        match event {
            Event::Mouse(ButtonReleased(mouse::Button::Left)) =>
                self.handle_click(layout, cursor_position, messages),
            Event::Touch(FingerPressed{id: _, position}) =>
                self.handle_click(layout, position, messages),
            _ => Status::Ignored,
        }
    }
}

impl<'a, Message: 'a+Clone> Into<Element<'a, Message>> for Collapsed<Message> {
    fn into(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

fn mouse_grabbed(layout: &ElementsLayout, position: Point) -> mouse::Interaction {
    if !layout.bounds.contains(position) {
        return mouse::Interaction::default();
    }
    if layout.title_bounds.contains(position) {
        return mouse::Interaction::Grab;
    }
    if position.x > layout.title_bounds.position().x + VLINE_WIDTH {
        return mouse::Interaction::Grab;
    }

    mouse::Interaction::default()
}

impl<Message: Clone> Collapsed<Message> {
    //
    fn handle_click(&self, layout: Layout, position: Point, messages: &mut Vec<Message>) -> Status {
        if !layout.bounds().contains(position) {
            return Status::Ignored;
        }
        let layout = self.layout_elements(layout);

        if layout.title_bounds.contains(position) {
            if let Some(message) = &self.on_title {
                messages.push(message.clone());
                return Status::Captured;
            }
        }

        if position.x > layout.title_bounds.position().x + VLINE_WIDTH {
            if let Some(message) = &self.on_plus {
                messages.push(message.clone());
                return Status::Captured;
            }
        }
        Status::Ignored
    }

    // https://docs.rs/iced_graphics/0.1.0/iced_graphics/enum.Primitive.html
    pub fn primitives(&self, layout: &ElementsLayout, draw_hline: bool) -> Vec<Primitive> {

        let (x1,x2,y) = layout.h_line;
        let h_line = h_line(x1, x2, y, WIDTH, Color::BLACK);

        let mesh = if draw_hline {
            let (y1,y2,x) = layout.v_line;
            let v_line = v_line(y1, y2, x, VLINE_WIDTH, Color::BLACK);
            merge_mesh2d(h_line, v_line)
        } else {
            h_line
        };

        let (x,y) = layout.plus;
        let plus = plus(x, y, PLUS_SIZE, PLUS_WIDTH, Color::BLACK);
        let mesh = merge_mesh2d(mesh, plus);

        let mesh = Primitive::Mesh2D{buffers: mesh, size: layout.bounds.size()};

        let title = text_left_aligned(
            self.title.clone(), 
            layout.title_bounds,
            TITLE_SIZE);

        let published = text_left_aligned(
            self.published.clone(), 
            layout.pub_bounds,
            META_SIZE);

        let duration = text_right_aligned(
            self.duration.clone(), 
            layout.dur_bounds,
            META_SIZE);
        
        vec![mesh, title, published, duration]
    }
}

