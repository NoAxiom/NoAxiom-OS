use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
    vec::{self, Vec},
};
use core::{
    any::Any,
    cmp::{max, min},
    convert::TryInto,
    mem::swap,
    str,
};

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::{Rgb888, RgbColor, WebColors},
    primitives::Rectangle,
    text,
};
use serde::Serialize;
use virtio_input_decoder::{Decoder, Key};

use crate::{
    lib_event::{Event, EventArgType, EventType},
    lib_request::Request,
    println,
    syscall::{getdents, open, read, utils::OpenFlags, write},
    utils::*,
};
pub const FONT_WIDTH: u32 = 6;
pub const FONT_HEIGHT: u32 = 12;
pub struct State {
    pub data: BTreeMap<String, String>,
}
pub trait Widget {
    fn get_id(&self) -> usize;
    fn set_id(&mut self, id: usize);
    fn set_position(&mut self, new_position: Point);
    fn get_position(&self) -> Point;
    fn set_size(&mut self, new_size: Size);
    fn get_size(&self) -> Size;
    fn set_margin(&mut self, new_margin: usize);
    fn get_margin(&self) -> usize;
    fn is_flex(&self) -> (bool, bool);
    fn on_event(
        &mut self,
        event: Event,
        pending_requests: &mut VecDeque<Request>,
        pending_actions: &mut VecDeque<String>,
        client_state: &mut State,
    );
    fn can_have_mouse_events(&self) -> bool;
    fn can_have_keyboard_events(&self) -> bool;
    fn draw(&self, draw_requests: &mut VecDeque<Request>, client_state: &State);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn on_action(&mut self, action: String, state: &mut State) {}
}
pub struct WidgetInputArea {
    id: usize,
    position: Point,
    size: Size,
    x_flex: bool,
    y_flex: bool,
    pub index: (usize, usize),
    margin: usize,
    bg_color: Rgb888,
    pub data: Vec<String>,
    action_map: BTreeMap<String, fn(&mut WidgetInputArea, &mut State)>,
    event_handler_map: BTreeMap<
        EventType,
        fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    >,
}
impl Widget for WidgetInputArea {
    fn set_position(&mut self, new_position: Point) {
        self.position = new_position;
    }
    fn set_size(&mut self, new_size: Size) {
        self.size = new_size;
    }
    fn get_position(&self) -> Point {
        self.position
    }
    fn get_size(&self) -> Size {
        self.size
    }
    fn is_flex(&self) -> (bool, bool) {
        (self.x_flex, self.y_flex)
    }
    fn draw(&self, draw_requests: &mut VecDeque<Request>, client_state: &State) {
        let area_width = self.get_size().width;
        let bound = self.get_position() + self.get_size();
        let char_per_line = (area_width / FONT_WIDTH) as usize;
        let font_size = Point::new(FONT_WIDTH as i32, FONT_HEIGHT as i32);
        let mut position = self.get_position();
        let request =
            Request::draw_rect(position, self.get_size(), 0, Rgb888::WHITE, self.bg_color);
        draw_requests.push_back(request);
        let mut char_count: usize = 0;
        let void_string = String::new();
        let (index_row, index_column) = self.index;
        'outer: for (line_count, line) in self.data.iter().enumerate() {
            let char_vec: Vec<char> = line.chars().collect();
            if char_vec.is_empty() {
                if index_row == line_count {
                    let offset = index_column;
                    let start = Point::new((offset as u32 * FONT_WIDTH) as i32, 0) + position;
                    let end = start + font_size.y_axis();
                    let request = Request::draw_line(start, end, 1, Rgb888::RED);
                    draw_requests.push_back(request);
                }
                position += Point::new(0, FONT_HEIGHT as i32);
                continue;
            }
            for (chunk_count, chunk) in char_vec.chunks(char_per_line).enumerate() {
                let text: String = chunk.iter().collect();
                if (position + font_size).y as i32 > bound.y {
                    break 'outer;
                }
                let request = Request::draw_text(text, position, false, Rgb888::BLACK);
                draw_requests.push_back(request);
                if index_row == line_count
                    && index_column >= chunk_count * char_per_line
                    && index_column < (chunk_count + 1) * char_per_line
                {
                    let offset = index_column - chunk_count * char_per_line;
                    let start = Point::new((offset as u32 * FONT_WIDTH) as i32, 0) + position;
                    let end = start + font_size.y_axis();
                    let request = Request::draw_line(start, end, 1, Rgb888::RED);
                    draw_requests.push_back(request);
                }
                position += Point::new(0, FONT_HEIGHT as i32);
            }
        }
    }
    fn on_event(
        &mut self,
        event: Event,
        pending_requests: &mut VecDeque<Request>,
        pending_actions: &mut VecDeque<String>,
        client_state: &mut State,
    ) {
        match event.event_type {
            crate::lib_event::EventType::MouseClick => {
                let click_position: Point = event.args[0].clone().try_into().unwrap();
                let relative_click_position = click_position - self.get_position();
                let area_width = self.get_size().width;
                let char_per_line = (area_width / FONT_WIDTH) as usize;
                let mut position = Point::zero();
                'outer: for (line_count, line) in self.data.iter().enumerate() {
                    let char_vec: Vec<char> = line.chars().collect();
                    if char_vec.is_empty() {
                        if relative_click_position.y >= position.y
                            && relative_click_position.y < position.y + FONT_HEIGHT as i32
                        {
                            self.index = (line_count, 0);
                            break;
                        }
                        position += Point::new(0, FONT_HEIGHT as i32);
                    }
                    for (chunk_count, chunk) in char_vec.chunks(char_per_line).enumerate() {
                        if relative_click_position.y >= position.y
                            && relative_click_position.y < position.y + FONT_HEIGHT as i32
                        {
                            let line_offset: usize = (relative_click_position.x as usize
                                + FONT_WIDTH as usize / 2)
                                / FONT_WIDTH as usize;
                            self.index = (
                                line_count,
                                min(line.len(), chunk_count * char_per_line + line_offset),
                            );
                            break 'outer;
                        }
                        position += Point::new(0, FONT_HEIGHT as i32);
                    }
                }
            }
            crate::lib_event::EventType::KeyDown => {
                let key_code: u32 = event.args[0].clone().try_into().unwrap();
                let key = virtio_input_decoder::Key::from_code(key_code as usize).unwrap();
                match key {
                    Key::BackSpace => {
                        let (index_row, index_column) = self.index;
                        if index_column > 0 {
                            let string = &mut self.data[index_row];
                            string.remove(index_column - 1);
                            self.index = (index_row, index_column - 1);
                        } else {
                            if index_row > 0 {
                                let string = self.data.remove(index_row);
                                self.data[index_row - 1].push_str(&string);
                                let str_len = self.data[index_row - 1].len();
                                self.index = (index_row - 1, str_len);
                            }
                        }
                    }
                    Key::Enter => {
                        let (index_row, index_column) = self.index;
                        let string = &mut self.data[index_row];
                        let new_string = string.split_off(index_column);
                        self.data.insert(index_row + 1, new_string);
                        self.index = (index_row + 1, 0);
                    }
                    _ => {
                        if let Ok(c) = Decoder::convert_key(key) {
                            let (index_row, index_column) = self.index;
                            let string = &mut self.data[index_row];
                            string.insert(index_column, c);
                            self.index = (index_row, index_column + 1);
                        }
                    }
                };
            }
            _ => {}
        }
        if let Some(handler) = self.event_handler_map.get(&event.event_type) {
            handler(self, event, pending_requests, pending_actions, client_state);
        }
    }
    fn can_have_keyboard_events(&self) -> bool {
        true
    }

    fn set_margin(&mut self, new_margin: usize) {
        self.margin = new_margin;
    }

    fn get_margin(&self) -> usize {
        self.margin
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn can_have_mouse_events(&self) -> bool {
        true
    }
    fn on_action(&mut self, action: String, state: &mut State) {
        if let Some(func) = self.action_map.get(&action) {
            func(self, state);
        }
    }
}
impl WidgetInputArea {
    pub fn new(one_line: bool) -> Self {
        let mut data: Vec<String> = Vec::new();
        data.push(String::new());
        if one_line {
            Self {
                id: 0,
                position: Point::new(0, 0),
                size: Size::new(0, 14),
                x_flex: true,
                y_flex: false,
                index: (0, 0),
                margin: 0,
                bg_color: Rgb888::WHITE,
                action_map: BTreeMap::new(),
                event_handler_map: BTreeMap::new(),
                data,
            }
        } else {
            Self {
                id: 0,
                position: Point::new(0, 0),
                size: Size::new(0, 0),
                x_flex: true,
                y_flex: true,
                index: (0, 0),
                margin: 0,
                bg_color: Rgb888::WHITE,
                action_map: BTreeMap::new(),
                event_handler_map: BTreeMap::new(),
                data,
            }
        }
    }
    pub fn register_action(&mut self, action: String, func: fn(&mut WidgetInputArea, &mut State)) {
        self.action_map.insert(action, func);
    }
    pub fn register_event_handler(
        &mut self,
        event_type: EventType,
        func: fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    ) {
        self.event_handler_map.insert(event_type, func);
    }
}
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}
pub struct WidgetBox {
    id: usize,
    position: Point,
    size: Size,
    x_flex: bool,
    y_flex: bool,
    pub layout: LayoutDirection,
    margin: usize,
    children: Vec<usize>,
    bg_color: Rgb888,
}
impl Widget for WidgetBox {
    fn set_position(&mut self, new_position: Point) {
        self.position = new_position;
    }
    fn set_size(&mut self, new_size: Size) {
        self.size = new_size;
    }
    fn get_position(&self) -> Point {
        self.position
    }
    fn get_size(&self) -> Size {
        self.size
    }
    fn is_flex(&self) -> (bool, bool) {
        (self.x_flex, self.y_flex)
    }
    fn draw(&self, draw_requests: &mut VecDeque<Request>, client_state: &State) {
        let position = self.get_position();
        let size = self.get_size();
        let request = Request::draw_rect(position, size, 0, Rgb888::WHITE, self.bg_color);
        draw_requests.push_back(request);
    }

    fn set_margin(&mut self, new_margin: usize) {
        self.margin = new_margin;
    }

    fn get_margin(&self) -> usize {
        self.margin
    }

    fn on_event(
        &mut self,
        event: Event,
        pending_requests: &mut VecDeque<Request>,
        pending_actions: &mut VecDeque<String>,
        client_state: &mut State,
    ) {
    }

    fn can_have_keyboard_events(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn can_have_mouse_events(&self) -> bool {
        false
    }
}
impl WidgetBox {
    pub fn new(layout: LayoutDirection, fix_size: Option<u32>) -> Self {
        if let Some(fix_size) = fix_size {
            let (x_flex, y_flex) = match layout {
                LayoutDirection::Vertical => (false, true),
                LayoutDirection::Horizontal => (true, false),
            };
            Self {
                id: 0,
                position: Point::new(0, 0),
                size: Size::new(fix_size, fix_size),
                x_flex,
                y_flex,
                layout,
                margin: 0,
                children: Vec::<usize>::new(),
                bg_color: Rgb888::CSS_SKY_BLUE,
            }
        } else {
            Self {
                id: 0,
                position: Point::new(0, 0),
                size: Size::new(0, 0),
                x_flex: true,
                y_flex: true,
                layout,
                margin: 0,
                children: Vec::<usize>::new(),
                bg_color: Rgb888::CSS_SKY_BLUE,
            }
        }
    }
    pub fn add_child(&mut self, child_id: usize) {
        self.children.push(child_id);
    }
    pub fn get_children(&self) -> &Vec<usize> {
        &self.children
    }
}

pub struct WidgetButton {
    id: usize,
    position: Point,
    size: Size,
    bg_color: Rgb888,
    label: String,
    margin: usize,
    event_handler: Option<fn(Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State)>,
}
impl Widget for WidgetButton {
    fn set_position(&mut self, new_position: Point) {
        self.position = new_position;
    }
    fn set_size(&mut self, new_size: Size) {
        self.size = new_size;
    }
    fn get_position(&self) -> Point {
        self.position
    }
    fn get_size(&self) -> Size {
        self.size
    }
    fn is_flex(&self) -> (bool, bool) {
        (false, false)
    }
    fn draw(&self, draw_requests: &mut VecDeque<Request>, client_state: &State) {
        let position = self.get_position();
        let size = self.get_size();
        let request = Request::draw_rect(position, size, 0, Rgb888::WHITE, self.bg_color);
        draw_requests.push_back(request);
        let request = Request::draw_text(
            self.label.clone(),
            position + size / 2 - Point::new(0, FONT_HEIGHT as i32 / 2),
            true,
            Rgb888::BLACK,
        );
        draw_requests.push_back(request);
    }

    fn set_margin(&mut self, new_margin: usize) {
        self.margin = new_margin;
    }

    fn get_margin(&self) -> usize {
        self.margin
    }

    fn on_event(
        &mut self,
        event: Event,
        pending_requests: &mut VecDeque<Request>,
        pending_actions: &mut VecDeque<String>,
        client_state: &mut State,
    ) {
        match event.event_type {
            EventType::MouseClick => {
                if let Some(event_handler) = self.event_handler {
                    event_handler(event, pending_requests, pending_actions, client_state);
                }
            }
            _ => {}
        }
    }

    fn can_have_keyboard_events(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn can_have_mouse_events(&self) -> bool {
        true
    }
}
impl WidgetButton {
    pub fn new(label: String) -> Self {
        Self {
            bg_color: Rgb888::GREEN,
            id: 0,
            position: Point::zero(),
            size: Size::new(36, 16),
            label,
            margin: 0,
            event_handler: None,
        }
    }
    pub fn register_handler(
        &mut self,
        handler: fn(Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    ) {
        self.event_handler = Some(handler);
    }
}
pub struct WidgetFileSelect {
    id: usize,
    position: Point,
    size: Size,
    bg_color: Rgb888,
    margin: usize,
    pub file_list: Vec<String>,
    pub select_on: Option<usize>,
    event_handler_map: BTreeMap<
        EventType,
        fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    >,
}
impl Widget for WidgetFileSelect {
    fn set_position(&mut self, new_position: Point) {
        self.position = new_position;
    }
    fn set_size(&mut self, new_size: Size) {
        self.size = new_size;
    }
    fn get_position(&self) -> Point {
        self.position
    }
    fn get_size(&self) -> Size {
        self.size
    }
    fn is_flex(&self) -> (bool, bool) {
        (true, true)
    }
    fn draw(&self, draw_requests: &mut VecDeque<Request>, client_state: &State) {
        let file_names = &self.file_list;
        let position = self.get_position();
        let size = self.get_size();
        let bound = position + size;
        let request = Request::draw_rect(position, size, 0, Rgb888::WHITE, self.bg_color);
        draw_requests.push_back(request);
        let mut text_position = position + Point::new(2, 0);
        for (index, file_name) in file_names.iter().enumerate() {
            if text_position.y + FONT_HEIGHT as i32 > bound.y {
                break;
            }
            if Some(index) == self.select_on {
                let rect_position = position.x_axis() + text_position.y_axis();
                let request = Request::draw_rect(
                    rect_position,
                    Size::new(size.width, FONT_HEIGHT),
                    0,
                    Rgb888::WHITE,
                    Rgb888::RED,
                );
                draw_requests.push_back(request);
            }
            let request =
                Request::draw_text(file_name.clone(), text_position, false, Rgb888::BLACK);
            text_position += Point::new(0, FONT_HEIGHT as i32);
            draw_requests.push_back(request);
        }
    }

    fn set_margin(&mut self, new_margin: usize) {
        self.margin = new_margin;
    }

    fn get_margin(&self) -> usize {
        self.margin
    }

    fn on_event(
        &mut self,
        event: Event,
        pending_requests: &mut VecDeque<Request>,
        pending_actions: &mut VecDeque<String>,
        client_state: &mut State,
    ) {
        match event.event_type {
            crate::lib_event::EventType::MouseClick => {
                let click_position: Point = event.args[0].clone().try_into().unwrap();
                let relative_click_position = click_position - self.get_position();

                let file_names = &self.file_list;
                let index: usize = relative_click_position.y as usize / FONT_HEIGHT as usize;
                if index < file_names.len() {
                    self.select_on = Some(index);
                }
            }
            _ => {}
        }
        if let Some(event_handler) = self.event_handler_map.get(&event.event_type) {
            event_handler(self, event, pending_requests, pending_actions, client_state);
        }
    }

    fn can_have_keyboard_events(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn can_have_mouse_events(&self) -> bool {
        true
    }
}
impl WidgetFileSelect {
    pub fn new() -> Self {
        let fd = open("/\0", OpenFlags::O_RDONLY);
        let file_names: Vec<String> = if fd > 0 {
            let mut buf = [0u8; 512];
            getdents(fd as usize, buf.as_mut_slice()).unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            bg_color: Rgb888::WHITE,
            id: 0,
            position: Point::zero(),
            size: Size::new(0, 0),
            margin: 0,
            file_list: file_names,
            select_on: None,
            event_handler_map: BTreeMap::new(),
        }
    }
    pub fn register_event_handler(
        &mut self,
        event_type: EventType,
        func: fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    ) {
        self.event_handler_map.insert(event_type, func);
    }
}

pub struct WidgetTextBox {
    id: usize,
    position: Point,
    size: Size,
    x_flex: bool,
    y_flex: bool,
    margin: usize,
    bg_color: Rgb888,
    action_map: BTreeMap<String, fn(&mut WidgetTextBox, &mut State)>,
    pub data: VecDeque<String>,
    event_handler_map: BTreeMap<
        EventType,
        fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    >,
}
impl Widget for WidgetTextBox {
    fn set_position(&mut self, new_position: Point) {
        self.position = new_position;
    }
    fn set_size(&mut self, new_size: Size) {
        self.size = new_size;
    }
    fn get_position(&self) -> Point {
        self.position
    }
    fn get_size(&self) -> Size {
        self.size
    }
    fn is_flex(&self) -> (bool, bool) {
        (self.x_flex, self.y_flex)
    }
    fn draw(&self, draw_requests: &mut VecDeque<Request>, client_state: &State) {
        let area_width = self.get_size().width;
        let area_height = self.get_size().height;
        let bound = self.get_position() + self.get_size();
        let char_per_line = (area_width / FONT_WIDTH) as usize;
        let font_size = Point::new(FONT_WIDTH as i32, FONT_HEIGHT as i32);
        let mut position = self.position;
        let line_count = area_height / FONT_HEIGHT;
        let start_line = max(0, self.data.len() as i32 - line_count as i32) as usize;
        let request =
            Request::draw_rect(position, self.get_size(), 0, Rgb888::WHITE, self.bg_color);
        draw_requests.push_back(request);
        for line in self.data.iter().skip(start_line) {
            let text_len = min(char_per_line, line.len());
            let text = line[..text_len].to_string();
            let request = Request::draw_text(text, position, false, Rgb888::BLACK);
            draw_requests.push_back(request);
            if position.y + FONT_HEIGHT as i32 > bound.y {
                break;
            }
            position += Point::new(0, FONT_HEIGHT as i32);
        }
    }
    fn on_event(
        &mut self,
        event: Event,
        pending_requests: &mut VecDeque<Request>,
        pending_actions: &mut VecDeque<String>,
        client_state: &mut State,
    ) {
    }
    fn can_have_keyboard_events(&self) -> bool {
        false
    }

    fn set_margin(&mut self, new_margin: usize) {
        self.margin = new_margin;
    }

    fn get_margin(&self) -> usize {
        self.margin
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn can_have_mouse_events(&self) -> bool {
        false
    }
    fn on_action(&mut self, action: String, state: &mut State) {
        if let Some(func) = self.action_map.get(&action) {
            func(self, state);
        }
    }
}

impl WidgetTextBox {
    pub fn new() -> Self {
        let mut data: Vec<String> = Vec::new();
        data.push(String::new());
        Self {
            id: 0,
            position: Point::new(0, 0),
            size: Size::new(0, 0),
            x_flex: true,
            y_flex: true,
            margin: 0,
            bg_color: Rgb888::WHITE,
            action_map: BTreeMap::new(),
            event_handler_map: BTreeMap::new(),
            data: VecDeque::new(),
        }
    }
    pub fn register_action(&mut self, action: String, func: fn(&mut WidgetTextBox, &mut State)) {
        self.action_map.insert(action, func);
    }
    pub fn register_event_handler(
        &mut self,
        event_type: EventType,
        func: fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    ) {
        self.event_handler_map.insert(event_type, func);
    }
}

enum PaintState {
    Idle,
    Moving,
    DrawingRect(Point, Point),
    DrawingLine(Point, Point),
    EnteringText(Point, String),
}
pub struct WidgetPaint {
    id: usize,
    position: Point,
    size: Size,
    x_flex: bool,
    y_flex: bool,
    margin: usize,
    bg_color: Rgb888,
    action_map: BTreeMap<String, fn(&mut WidgetPaint, &mut State)>,
    pub data: Vec<Box<PaintElement>>,
    state: PaintState,
    fb_size: Size,
    fb_offset: Point,
    event_handler_map: BTreeMap<
        EventType,
        fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    >,
    current_tool: PaintTool,
}

enum PaintTool {
    Move,
    Rect,
    Line,
    Text,
}

impl Widget for WidgetPaint {
    fn set_position(&mut self, new_position: Point) {
        self.position = new_position;
    }
    fn set_size(&mut self, new_size: Size) {
        self.size = new_size;
    }
    fn get_position(&self) -> Point {
        self.position
    }
    fn get_size(&self) -> Size {
        self.size
    }
    fn is_flex(&self) -> (bool, bool) {
        (self.x_flex, self.y_flex)
    }
    fn draw(&self, draw_requests: &mut VecDeque<Request>, client_state: &State) {
        let position = self.get_position();
        let size = self.get_size();
        let (fb_clipped_position, fb_clipped_size) = self.get_clipped_fb();
        let request = Request::draw_rect(position, size, 0, Rgb888::WHITE, self.bg_color);
        draw_requests.push_back(request);
        let request = Request::draw_rect(
            fb_clipped_position,
            fb_clipped_size,
            0,
            Rgb888::WHITE,
            Rgb888::WHITE,
        );
        draw_requests.push_back(request);

        for e in &self.data {
            let request = e.to_request(
                self.fb_offset + self.get_position(),
                fb_clipped_position,
                fb_clipped_size,
                position,
            );
            draw_requests.push_back(request);
        }
    }
    fn on_event(
        &mut self,
        event: Event,
        pending_requests: &mut VecDeque<Request>,
        pending_actions: &mut VecDeque<String>,
        client_state: &mut State,
    ) {
        fn rect_from_two_points(p1: Point, p2: Point) -> DrawElementRect {
            let p3 = p1.component_min(p2);
            let p4 = p1.component_max(p2);
            let p = p4 - p3;
            let size = Size::new(p.x as u32, p.y as u32);
            DrawElementRect {
                start: p3,
                size,
                stoke_width: 1,
                stoke_color: Rgb888::BLACK,
                fill_color: Rgb888::WHITE,
            }
        }
        match self.current_tool {
            PaintTool::Move => match event.event_type {
                EventType::MouseDrag => {
                    let shift: Point = event.args[0].clone().try_into().unwrap();
                    self.fb_offset += shift;
                }
                EventType::MousePress => {
                    self.state = PaintState::Moving;
                }
                EventType::MouseRelease => {
                    self.state = PaintState::Idle;
                }
                _ => {}
            },
            PaintTool::Rect => match event.event_type {
                EventType::MouseDrag => {
                    let shift: Point = event.args[0].clone().try_into().unwrap();
                    match self.state {
                        PaintState::DrawingRect(p1, ref mut p2) => {
                            *p2 += shift;
                            self.data.pop();
                            self.data
                                .push(Box::new(PaintElement::Rect(rect_from_two_points(
                                    p1,
                                    p2.clone(),
                                ))))
                        }
                        _ => {}
                    }
                }
                EventType::MousePress => {
                    let press_position = event.args[0].clone().try_into().unwrap();
                    let (fb_position, fb_size) = self.get_clipped_fb();
                    let fb = Rectangle::new(fb_position, fb_size);
                    if fb.contains(press_position) {
                        self.state = PaintState::DrawingRect(
                            press_position - fb_position,
                            press_position - fb_position,
                        );
                        self.data
                            .push(Box::new(PaintElement::Rect(rect_from_two_points(
                                press_position,
                                press_position,
                            ))))
                    }
                }
                EventType::MouseRelease => {
                    self.state = PaintState::Idle;
                }
                _ => {}
            },
            PaintTool::Line => match event.event_type {
                EventType::MouseDrag => {
                    let shift: Point = event.args[0].clone().try_into().unwrap();
                    match self.state {
                        PaintState::DrawingLine(p1, ref mut p2) => {
                            *p2 += shift;
                            self.data.pop();
                            let line = DrawElementLine {
                                start: p1,
                                end: p2.clone(),
                                stoke_width: 1,
                                stoke_color: Rgb888::BLACK,
                            };
                            self.data.push(Box::new(PaintElement::Line(line)));
                        }
                        _ => {}
                    }
                }
                EventType::MousePress => {
                    let press_position = event.args[0].clone().try_into().unwrap();
                    let (fb_position, fb_size) = self.get_clipped_fb();
                    let fb = Rectangle::new(fb_position, fb_size);
                    if fb.contains(press_position) {
                        self.state = PaintState::DrawingLine(
                            press_position - fb_position,
                            press_position - fb_position,
                        );
                        let line = DrawElementLine {
                            start: press_position - fb_position,
                            end: press_position,
                            stoke_width: 1,
                            stoke_color: Rgb888::BLACK,
                        };
                        self.data.push(Box::new(PaintElement::Line(line)));
                    }
                }
                EventType::MouseRelease => {
                    self.state = PaintState::Idle;
                }
                _ => {}
            },
            PaintTool::Text => match event.event_type {
                EventType::MouseClick => {
                    let click_position = event.args[0].clone().try_into().unwrap();
                    let (fb_position, fb_size) = self.get_clipped_fb();
                    let fb = Rectangle::new(fb_position, fb_size);
                    if fb.contains(click_position) {
                        self.state =
                            PaintState::EnteringText(click_position - fb_position, String::new());
                        let text = DrawElementText {
                            position: click_position - fb_position,
                            text: "input text".to_string(),
                            centered: false,
                            color: Rgb888::BLACK,
                        };
                        self.data.push(Box::new(PaintElement::Text(text)));
                    }
                }
                EventType::KeyDown => {
                    let key_code: u32 = event.args[0].clone().try_into().unwrap();
                    let key = virtio_input_decoder::Key::from_code(key_code as usize).unwrap();
                    match self.state {
                        PaintState::EnteringText(position, ref mut string) => {
                            let mut complete: bool = false;
                            match key {
                                Key::BackSpace => {
                                    string.pop();
                                }
                                Key::Enter => complete = true,
                                _ => {
                                    if let Ok(c) = Decoder::convert_key(key) {
                                        string.push(c);
                                    }
                                }
                            };
                            self.data.pop();
                            let text = DrawElementText {
                                position,
                                text: string.clone(),
                                centered: false,
                                color: Rgb888::BLACK,
                            };
                            self.data.push(Box::new(PaintElement::Text(text)));
                            if complete {
                                self.state = PaintState::Idle;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
        }
    }
    fn can_have_keyboard_events(&self) -> bool {
        true
    }

    fn set_margin(&mut self, new_margin: usize) {
        self.margin = new_margin;
    }

    fn get_margin(&self) -> usize {
        self.margin
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn can_have_mouse_events(&self) -> bool {
        true
    }
    fn on_action(&mut self, action: String, state: &mut State) {
        if let Some(func) = self.action_map.get(&action) {
            func(self, state);
        }
    }
}
impl WidgetPaint {
    pub fn new(fb_size: Size) -> Self {
        Self {
            id: 0,
            position: Point::new(0, 0),
            size: Size::new(0, 0),
            x_flex: true,
            y_flex: true,
            margin: 0,
            bg_color: Rgb888::CSS_SLATE_GRAY,
            action_map: BTreeMap::new(),
            event_handler_map: BTreeMap::new(),
            data: Vec::new(),
            fb_size,
            current_tool: PaintTool::Move,
            state: PaintState::Idle,
            fb_offset: Point::zero(),
        }
    }
    pub fn register_action(&mut self, action: String, func: fn(&mut WidgetPaint, &mut State)) {
        self.action_map.insert(action, func);
    }
    pub fn register_event_handler(
        &mut self,
        event_type: EventType,
        func: fn(&mut Self, Event, &mut VecDeque<Request>, &mut VecDeque<String>, &mut State),
    ) {
        self.event_handler_map.insert(event_type, func);
    }
    fn get_clipped_fb(&self) -> (Point, Size) {
        fn size_from_two_point(p1: Point, p2: Point) -> Size {
            let p = p2 - p1;
            Size::new(p.x as u32, p.y as u32)
        }
        let fb_position = self.fb_offset + self.get_position();
        let fb_size = self.fb_size;
        let fb_clipped_topleft: Point = fb_position.component_max(Point::zero());
        let fb_clipped_bottomright: Point =
            (fb_position + fb_size).component_min(Point::zero() + self.get_size());
        let fb_clipped_size: Size = size_from_two_point(fb_clipped_topleft, fb_clipped_bottomright);
        (fb_clipped_topleft, fb_clipped_size)
    }
    pub fn update_tool(&mut self, new_tool: &String) {
        self.state = PaintState::Idle;
        match new_tool.as_str() {
            "Move" => self.current_tool = PaintTool::Move,
            "Rect" => self.current_tool = PaintTool::Rect,
            "Line" => self.current_tool = PaintTool::Line,
            "Text" => self.current_tool = PaintTool::Text,
            _ => {
                unreachable!()
            }
        }
    }
    pub fn undo(&mut self) {
        self.data.pop();
        self.state = PaintState::Idle;
    }
    pub fn clear(&mut self) {
        self.data.clear();
        self.state = PaintState::Idle;
    }
    pub fn save_file(&self, mut file_name: String) {
        file_name.push('\0');
        let data: String = serde_json::to_string(&self.data).unwrap();
        let fd = open(&file_name, OpenFlags::O_RDWR | OpenFlags::O_CREATE);
        println!("save_file");
        if fd >= 0 {
            let len = write(fd as usize, data.as_bytes()) as usize;
            println!("write file len:{:?}", len);
        };

        println!("SAVE SUCCESS:{}", data);
    }
    pub fn open_file(&mut self, mut file_name: String) {
        file_name.push('\0');
        let fd = open(&file_name, OpenFlags::O_RDWR);
        if fd >= 0 {
            let mut buf = Box::new([0u8; 4096]);
            let len = read(fd as usize, buf.as_mut_slice()) as usize;
            let file_data = str::from_utf8(&buf.as_slice()[..len]).unwrap();
            let mut vec_data: Vec<Box<PaintElement>> = serde_json::from_str(file_data).unwrap();
            self.data.clear();
            self.data.append(&mut vec_data);
        };
    }
}
// const DATA: &str =
// r#"[{"Rect":{"start":{"x":16,"y":21},"size":{"width":72,"height":33},"
// stoke_width":1,"stoke_color":{"r":0,"g":0,"b":0},"fill_color":{"r":255,"g":
// 255,"b":255}}},{"Line":{"start":{"x":90,"y":29},"end":{"x":2,"y":77},"
// stoke_width":1,"stoke_color":{"r":0,"g":0,"b":0}}}]"#;
