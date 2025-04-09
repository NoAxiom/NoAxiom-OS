use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{cmp::min, mem::swap, ops::BitXor};

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::{IntoStorage, Rgb888},
    prelude::RgbColor,
    primitives::Rectangle,
};
use enum_dispatch::enum_dispatch;
use serde_derive::{Deserialize, Serialize};

use crate::{
    lib_request::Request,
    lib_widget::{FONT_HEIGHT, FONT_WIDTH},
};
#[derive(Serialize, Deserialize)]
#[serde(remote = "Point")]
pub struct PointDef {
    /// The x coordinate.
    pub x: i32,

    /// The y coordinate.
    pub y: i32,
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "Size")]
pub struct SizeDef {
    /// The width.
    pub width: u32,

    /// The height.
    pub height: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "Rgb888")]
pub struct Rgb888Def {
    #[serde(getter = "Rgb888::r")]
    r: u8,
    #[serde(getter = "Rgb888::g")]
    g: u8,
    #[serde(getter = "Rgb888::b")]
    b: u8,
}
impl From<Rgb888Def> for Rgb888 {
    fn from(c: Rgb888Def) -> Self {
        Rgb888::new(c.r, c.g, c.b)
    }
}
#[derive(Serialize, Deserialize)]
pub struct DrawElementRect {
    #[serde(with = "PointDef")]
    pub start: Point,
    #[serde(with = "SizeDef")]
    pub size: Size,
    pub stoke_width: u32,
    #[serde(with = "Rgb888Def")]
    pub stoke_color: Rgb888,
    #[serde(with = "Rgb888Def")]
    pub fill_color: Rgb888,
}
#[derive(Serialize, Deserialize)]
pub struct DrawElementLine {
    #[serde(with = "PointDef")]
    pub start: Point,
    #[serde(with = "PointDef")]
    pub end: Point,
    pub stoke_width: u32,
    #[serde(with = "Rgb888Def")]
    pub stoke_color: Rgb888,
}
#[derive(Serialize, Deserialize)]
pub struct DrawElementText {
    #[serde(with = "PointDef")]
    pub position: Point,
    pub text: String,
    pub centered: bool,
    #[serde(with = "Rgb888Def")]
    pub color: Rgb888,
}
#[enum_dispatch(PaintElement)]
pub trait ToRequst {
    fn to_request(
        &self,
        fb_position: Point,
        fb_clipped_position: Point,
        fb_clipped_size: Size,
        widget_position: Point,
    ) -> Request;
}

#[derive(Serialize, Deserialize)]
#[enum_dispatch]
pub enum PaintElement {
    Rect(DrawElementRect),
    Line(DrawElementLine),
    Text(DrawElementText),
}

impl ToRequst for DrawElementLine {
    fn to_request(
        &self,
        fb_position: Point,
        fb_clipped_position: Point,
        fb_clipped_size: Size,
        widget_position: Point,
    ) -> Request {
        let fb_area = Rectangle::new(fb_clipped_position, fb_clipped_size);
        let line = Line::new(self.start + fb_position, self.end + fb_position);
        let stoke_color = if segment_interact_rect(&line, &fb_area) {
            Rgb888::RED
        } else {
            self.stoke_color
        };
        let line_clipped = clip_segment(&line, &fb_area);
        Request::draw_line(
            line_clipped.start,
            line_clipped.end,
            self.stoke_width,
            stoke_color,
        )
    }
}
impl ToRequst for DrawElementRect {
    fn to_request(
        &self,
        fb_position: Point,
        fb_clipped_position: Point,
        fb_clipped_size: Size,
        widget_position: Point,
    ) -> Request {
        let fb_area = Rectangle::new(fb_clipped_position, fb_clipped_size);
        let rect = Rectangle::new(self.start + fb_position, self.size);
        let rect_clipped = rect.intersection(&fb_area);
        Request::draw_rect(
            rect_clipped.top_left + widget_position,
            rect_clipped.size,
            self.stoke_width,
            self.stoke_color,
            self.fill_color,
        )
    }
}
impl ToRequst for DrawElementText {
    fn to_request(
        &self,
        fb_position: Point,
        fb_clipped_position: Point,
        fb_clipped_size: Size,
        widget_position: Point,
    ) -> Request {
        let fb_area = Rectangle::new(fb_clipped_position, fb_clipped_size);
        let text_position = self.position + fb_position;
        let mut text_end: usize = self.text.len();
        let mut text_start: usize = 0;
        if text_position.y + FONT_HEIGHT as i32 > (fb_clipped_position + fb_clipped_size).y {
            text_end = 0;
        }
        if text_position.y < fb_clipped_position.y {
            text_end = 0;
        }
        if text_position.x < fb_clipped_position.x {
            text_start = ((fb_clipped_position.x - text_position.x) as usize + FONT_WIDTH as usize
                - 1)
                / FONT_WIDTH as usize;
        }

        text_end = min(
            text_end,
            ((fb_clipped_position + fb_clipped_size).x - text_position.x) as usize
                / FONT_WIDTH as usize,
        );
        text_start = min(text_start, text_end);
        Request::draw_text(
            self.text[text_start..text_end].to_string(),
            text_position + Point::new(text_start as i32 * FONT_WIDTH as i32, 0),
            self.centered,
            self.color,
        )
    }
}

#[derive(Clone)]
pub struct Line {
    pub start: Point,
    pub end: Point,
}
impl Line {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }
}
pub fn cross_product(p1: &Point, p2: &Point) -> i32 {
    p1.x * p2.y - p2.x * p1.y
}

pub fn segment_interact(l1: &Line, l2: &Line) -> bool {
    let mut a = l1.end - l1.start;
    let mut b = l2.end - l2.start;
    let mut t = l2.start - l1.start;
    if cross_product(&a, &b) < 0 {
        swap(&mut a, &mut b);
        t = l1.start - l2.start;
    }
    let product = cross_product(&a, &b);
    let t_cross_a = cross_product(&t, &a);
    let t_cross_b = cross_product(&t, &b);
    if product == 0 {
        return false;
    } else if product > 0 {
        t_cross_a >= 0 && t_cross_a <= product && t_cross_b >= 0 && t_cross_b <= product
    } else {
        unreachable!()
    }
}
pub fn segment_interact_rect(line: &Line, rect: &Rectangle) -> bool {
    let top_left = rect.top_left;
    let top_right = rect.top_left + rect.size.x_axis();
    let bottom_left = rect.top_left + rect.size.y_axis();
    let bottom_right = rect.top_left + rect.size;
    let e1 = Line::new(top_left, top_right);
    let e2 = Line::new(top_right, bottom_right);
    let e3 = Line::new(bottom_right, bottom_left);
    let e4 = Line::new(bottom_left, top_left);
    segment_interact(line, &e1)
        || segment_interact(line, &e2)
        || segment_interact(line, &e3)
        || segment_interact(line, &e4)
}
pub fn segment_interation(s1: &Line, s2: &Line) -> Point {
    let a = s1.end - s1.start;
    let b = s2.end - s2.start;
    let c = s2.start - s1.start;
    let product = (cross_product(&a, &b)).abs();
    s2.start + b * cross_product(&a, &c).abs() / product
}
pub fn segment_rect_interaction(line: &Line, rect: &Rectangle) -> (Option<Point>, Option<Point>) {
    let top_left = rect.top_left;
    let top_right = rect.top_left + rect.size.x_axis();
    let bottom_left = rect.top_left + rect.size.y_axis();
    let bottom_right = rect.top_left + rect.size;
    let e1 = Line::new(top_left, top_right);
    let e2 = Line::new(top_right, bottom_right);
    let e3 = Line::new(bottom_right, bottom_left);
    let e4 = Line::new(bottom_left, top_left);
    let mut interations: Vec<Point> = Vec::new();
    if segment_interact(line, &e1) {
        interations.push(segment_interation(line, &e1));
    }
    if segment_interact(line, &e2) {
        interations.push(segment_interation(line, &e2));
    }
    if segment_interact(line, &e3) {
        interations.push(segment_interation(line, &e3));
    }
    if segment_interact(line, &e4) {
        interations.push(segment_interation(line, &e4));
    }

    if interations.len() > 2 {
        warn!("duplicated points");
    }
    let p1 = interations.get(0).cloned();
    let p2 = interations.get(1).cloned();
    (p1, p2)
}
pub fn clip_segment(line: &Line, rect: &Rectangle) -> Line {
    if !rect.contains(line.start) && !rect.contains(line.end) {
        if let (Some(p1), Some(p2)) = segment_rect_interaction(line, rect) {
            Line::new(p1, p2)
        } else {
            Line::new(rect.top_left, rect.top_left)
        }
    } else if rect.contains(line.start) && rect.contains(line.end) {
        line.clone()
    } else {
        let mut p1 = line.start;
        let mut p2 = line.end;
        if rect.contains(p2) {
            swap(&mut p1, &mut p2);
        }
        let p3 = segment_rect_interaction(line, rect).0.unwrap();
        Line::new(p1, p3)
    }
}
