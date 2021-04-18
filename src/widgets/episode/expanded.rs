use iced::{Length, Element};
use iced_native::{Widget, layout, Hasher, mouse, Size, Layout, Point, Rectangle, Clipboard};
use iced_native::event::{Event, Status};
use iced_native::text::Renderer as _;
use iced_graphics::{Vector, backend, Backend, Defaults, Primitive, Renderer, Color, Font};
use std::cell::Cell;

use super::collapsed::{self, Collapsed, MARGIN, META_SIZE, WIDTH};
use super::super::util::{h_line, text_left_aligned, merge_mesh2d};

const DESCRIPTION_SIZE: f32 = 18.0;
const LOWER_H_SPACE: f32 = DESCRIPTION_SIZE/2.0;

#[derive(Debug)]
pub struct Expanded<Message> {
    collapsed_height: Cell<Option<f32>>,
    collapsed: Collapsed<Message>,
    description: String,
    on_stream: Option<Message>,
    on_add: Option<Message>,
    on_remove: Option<Message>,
    on_disk: bool,
}

struct ElementsLayout {
    bounds: Rectangle,
    collapsed: collapsed::ElementsLayout,
    mid_line: (f32,f32,f32),
    description_bounds: Rectangle,
    buttons_bounds: Rectangle,
    bottom_line: (f32,f32,f32),
}

impl<Message> Expanded<Message> {
    fn elements(&self, bounds: Rectangle, width: f32, height: f32) -> ElementsLayout {
        let y = self.collapsed_height.get().unwrap();
        let x = 0.0;

        let description_bounds = Rectangle {x, y, 
            width: width - MARGIN,
            height};

        let buttons_bounds = Rectangle {x, 
            y: height-META_SIZE-WIDTH, 
            width: description_bounds.width, 
            height: 1.0*META_SIZE};

        let mid_line = (x, width, self.collapsed_height.get().unwrap() - WIDTH);

        let x = 0.0;
        let bottom_line = (x, width, height - WIDTH);

        let height = self.collapsed_height.get().unwrap();
        let collapsed = self.collapsed
            .elements(bounds, width, height, 0., 0.);

        ElementsLayout {
            bounds,
            collapsed,
            description_bounds,
            buttons_bounds,
            mid_line,
            bottom_line
        }
    }
}

impl<Message> Expanded<Message> {
    pub fn from_collapsed(collapsed: Collapsed<Message>, mut description: String) -> Self {
        description.truncate(400);
        Self {
            collapsed,
            collapsed_height: Cell::new(None),
            description,
            on_stream: None,
            on_add: None,
            on_remove: None,
            on_disk: false,
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

impl<Message: Clone, B> Widget<Message, Renderer<B>> for Expanded<Message>
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
        let layout = self.collapsed.layout(renderer, limits);
        let Size {width, height: mut needed_height} = layout.size();
        let Size {height, ..} = limits.max();
        self.collapsed_height.set(Some(needed_height));

        let text_bounds = Size::new(width - MARGIN, height);
        let (_, text_height) = renderer.measure(&self.description, DESCRIPTION_SIZE as u16, Font::Default, text_bounds);
        needed_height += text_height;
        needed_height += 1.2*META_SIZE;
        needed_height += WIDTH;
        needed_height += LOWER_H_SPACE;

        layout::Node::new(Size::new(width, needed_height))
    }
    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash;
        self.collapsed.title.hash(state);
        1.hash(state); // force different hash from collapsed
    }
    fn draw(&self, 
        _renderer: &mut Renderer<B>, 
        _defaults: &Defaults, 
        layout: Layout<'_>, 
        cursor_position: Point, 
        _viewport: &Rectangle
    ) -> (Primitive, mouse::Interaction) {
        // TODO meta bounds
        let Rectangle {width, height, x, y} = layout.bounds();
        let elements = self.elements(layout.bounds(), width, height);
        let mouse = mouse_grabbed(&elements, cursor_position);

        let primitives = self.primitives(&elements);
        let primitives = Primitive::Group{primitives};
        let primitives = Primitive::Translate {
            translation: Vector::new(x, y),
            content: Box::new(primitives)
        };

        (primitives, mouse::Interaction::default())
    }
    fn on_event(&mut self, 
        event: Event, 
        layout: Layout<'_>, 
        cursor_position: Point, 
        _renderer: &Renderer<B>, 
        _clipboard: &mut dyn Clipboard,
        messages: &mut Vec<Message>, 
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

impl<'a, Message: 'a+Clone> Into<Element<'a, Message>> for Expanded<Message> {
    fn into(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

fn mouse_grabbed(_layout: &ElementsLayout, _position: Point) -> mouse::Interaction {
    /* if !layout.bounds.contains(position) {
        return mouse::Interaction::default();
    }
    if layout.title_bounds.contains(position) {
        return mouse::Interaction::Grab;
    }
    if position.x > layout.title_bounds.position().x + VLINE_WIDTH {
        return mouse::Interaction::Grab;
    } */

    mouse::Interaction::default()
}

impl<Message: Clone> Expanded<Message> {
    fn handle_click(&self, layout: Layout, position: Point, messages: &mut Vec<Message>) -> Status {
        if !layout.bounds().contains(position) {
            return Status::Ignored;
        }
        let Rectangle {width, height, x, y} = layout.bounds();
        let elements = self.elements(layout.bounds(), width, height);

        /* if elements.title_bounds.contains(position) {
            todo!("add collapse logic");
            /* messages.push(msg.clone());
            return Status::Captured; */
        }

        if position.x > elements.title_bounds.position().x + VLINE_WIDTH {
            if let Some(msg) = &self.collapsed.on_plus {
                messages.push(msg.clone());
                return Status::Captured;
            }
        } */
        Status::Ignored
    }

    // https://docs.rs/iced_graphics/0.1.0/iced_graphics/enum.Primitive.html
    fn primitives(&self, layout: &ElementsLayout) -> Vec<Primitive> {

        let mut primitives = self.collapsed.primitives(&layout.collapsed, false);
        let (x1,x2,y) = layout.mid_line;
        let mid = h_line(x1, x2, y, WIDTH, Color::BLACK);
        let (x1,x2,y) = layout.bottom_line;
        let bottom = h_line(x1, x2, y, WIDTH, Color::BLACK);
        let mesh = merge_mesh2d(mid, bottom);
        let mesh = Primitive::Mesh2D{buffers: mesh, size: layout.bounds.size()};
        primitives.push(mesh);

        let description = text_left_aligned(
            self.description.clone(),
            layout.description_bounds,
            DESCRIPTION_SIZE);
        primitives.push(description);

        let text = match self.on_disk {
            true => "stream | add | remove",
            false => "stream | add",
        };

        let buttons = text_left_aligned(
            text.into(), 
            layout.buttons_bounds,
            META_SIZE);
        primitives.push(buttons);
        primitives
    }
}
