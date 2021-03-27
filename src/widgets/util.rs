use iced::HorizontalAlignment;
use iced_native::Rectangle;
use iced_graphics::{Primitive, Color, Font};

/// horizontal line from x1 to x2 at height y
use iced_graphics::triangle::{Vertex2D, Mesh2D};
pub fn h_line(x1: f32, x2: f32, y: f32, width: f32, color: Color) -> Mesh2D {
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
pub fn v_line(y1: f32, y2: f32, x: f32, width: f32, color: Color) -> Mesh2D {
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

pub fn text_left_aligned(text: String, bounds: Rectangle, size: f32) -> Primitive {
    text_aligned(text, bounds, size, HorizontalAlignment::Left)
}
pub fn text_right_aligned(text: String, bounds: Rectangle, size: f32) -> Primitive {
    text_aligned(text, bounds, size, HorizontalAlignment::Right)
}
fn text_aligned(text: String, bounds: Rectangle, size: f32, horizontal_alignment: HorizontalAlignment) -> Primitive {
    Primitive::Text {
        content: text,
        bounds,
        color: Color::BLACK,
        size,
        font: Font::Default,
        horizontal_alignment,
        vertical_alignment: iced::VerticalAlignment::Top,
    }
}

use std::iter::Extend;
pub fn merge_mesh2d(mut a: Mesh2D, mut b: Mesh2D) -> Mesh2D {
    let offset = a.vertices.len() as u32;
    a.vertices.append(&mut b.vertices);
    a.indices.extend(b.indices.drain(..).map(|b| b+offset));
    a
}

/// draws a plus centred at x,y
pub fn plus(x: f32, y: f32, size: f32, stroke: f32, color: Color) -> Mesh2D {
    let r = size/2.0;
    let h_line = h_line(x-r, x+r, y-stroke/2.0, stroke, color);
    let v_line = v_line(y-r, y+r, x-stroke/2.0, stroke, color);
    merge_mesh2d(h_line, v_line)
}
