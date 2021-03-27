use iced::{Length, Element};
use iced_native::{Widget, layout, Hasher, mouse, Size, Layout, Point, Rectangle, Clipboard};
use iced_native::event::{Event, Status};
use iced_native::text::Renderer as _;
use iced_graphics::{Vector, backend, Backend, Defaults, Primitive, Renderer, Color, Font};

use super::collapsed::{self, Collapsed, MARGIN, META_SIZE, WIDTH};
use super::super::util::{h_line, text_left_aligned, merge_mesh2d};

const DESCRIPTION_SIZE: f32 = 18.0;

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
    collapsed: collapsed::ElementsLayout,
    mid_line: (f32,f32,f32),
    description_bounds: Rectangle,
    buttons_bounds: Rectangle,
    bottom_line: (f32,f32,f32),
}

impl<Message> Expanded<Message> {
    fn layout_elements(&self, layout: Layout) -> ElementsLayout {
        let Rectangle {width, height, ..} = layout.bounds();

        let collapsed = self.collapsed.layout_elements(layout);
        let y = collapsed.bounds.y;
        let x = MARGIN;

        let description_bounds = Rectangle {x, y, 
            width: width - MARGIN,
            height};

        let buttons_bounds = Rectangle {x, 
            y: height-META_SIZE-WIDTH, 
            width: description_bounds.width, 
            height: 1.0*META_SIZE};

        let mid_line = (x, width, collapsed.bounds.height);

        let x = 0.0;
        let bottom_line = (x, width, height - WIDTH);

        ElementsLayout {
            bounds: layout.bounds(),
            collapsed,
            description_bounds,
            buttons_bounds,
            mid_line,
            bottom_line
        }
    }
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

        let text_bounds = Size::new(width - MARGIN, height);
        let (_, text_height) = renderer.measure(&self.description, DESCRIPTION_SIZE as u16, Font::Default, text_bounds);
        needed_height += text_height;
        needed_height += 1.2*META_SIZE;
        needed_height += WIDTH;

        layout::Node::new(Size::new(width, needed_height))
    }
    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash;
        self.collapsed.title.hash(state);
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

        let primitives = self.primitives(&layout);
        let primitives = Primitive::Group{primitives};
        let primitives = Primitive::Translate {
            translation: Vector::new(layout.bounds.x, layout.bounds.y),
            content: Box::new(primitives)
        };

        (primitives, mouse::Interaction::default())
    }
    fn on_event(&mut self, 
        event: Event, 
        layout: Layout<'_>, 
        cursor_position: Point, 
        messages: &mut Vec<Message>, 
        _renderer: &Renderer<B>, 
        _clipboard: Option<&dyn Clipboard>
    ) -> Status {
        // use mouse::Event::ButtonReleased;
        // use iced_native::touch::Event::FingerPressed;

        // match event {
        //     Event::Mouse(ButtonReleased(mouse::Button::Left)) =>
        //         self.handle_click(layout, cursor_position, messages),
        //     Event::Touch(FingerPressed{id: _, position}) =>
        //         self.handle_click(layout, position, messages),
        //     _ => Status::Ignored,
        // }
        Status::Ignored
    }
}

impl<'a, Message: 'a+Clone> Into<Element<'a, Message>> for Expanded<Message> {
    fn into(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

impl<Message: Clone> Expanded<Message> {
    //
    // fn handle_click(&self, layout: Layout, position: Point, messages: &mut Vec<Message>) -> Status {
    //     if !layout.bounds().contains(position) {
    //         return Status::Ignored;
    //     }
    //     let layout = self.layout_elements(layout);

    //     if layout.title_bounds.contains(position) {
    //         if let Some(message) = &self.on_title {
    //             messages.push(message.clone());
    //             return Status::Captured;
    //         }
    //     }

    //     if position.x > layout.title_bounds.position().x + VLINE_WIDTH {
    //         if let Some(message) = &self.on_plus {
    //             messages.push(message.clone());
    //             return Status::Captured;
    //         }
    //     }
    //     Status::Ignored
    // }

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

        let buttons = text_left_aligned(
            "stream | add | remove".into(), 
            layout.buttons_bounds,
            META_SIZE);
        primitives.push(buttons);
        primitives
    }
}
