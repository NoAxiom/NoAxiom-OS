use alloc::{
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
};

use syscall::{read, write};

use super::*;
#[macro_use]
use alloc::vec;
use alloc::vec::Vec;
use core::{convert::TryInto, mem::discriminant, str};

use derive_more::{From, Into, TryInto};
use embedded_graphics::geometry::{Point, Size};
use lazy_static::lazy_static;
const SOCKET_BUFFER_SIZE: usize = 256;
#[derive(Ord, Eq, PartialEq, PartialOrd, Debug)]
pub enum EventType {
    NewWindow,
    WindowClose,
    MouseClick,
    MouseDrag,
    MenuItem,
    KeyDown,
    HeatBeat,
    Resize,
    PopupReponse,
    MousePress,
    MouseRelease,
    Shutdown,
}
#[derive(Debug, Clone, TryInto)]
pub enum EventArgType {
    Id(u32),
    Point(Point),
    U32(u32),
    Size(Size),
    String(String),
}
impl EventType {
    pub fn encode(&self) -> u32 {
        match self {
            EventType::NewWindow => 1,
            EventType::WindowClose => 2,
            EventType::MouseClick => 3,
            EventType::MouseDrag => 4,
            EventType::MenuItem => 5,
            EventType::KeyDown => 6,
            EventType::HeatBeat => 7,
            EventType::Resize => 8,
            EventType::PopupReponse => 9,
            EventType::MousePress => 10,
            EventType::MouseRelease => 11,
            EventType::Shutdown => 12,
        }
    }
    pub fn decode(code: u32) -> Self {
        match code {
            1 => EventType::NewWindow,
            2 => EventType::WindowClose,
            3 => EventType::MouseClick,
            4 => EventType::MouseDrag,
            5 => EventType::MenuItem,
            6 => EventType::KeyDown,
            7 => EventType::HeatBeat,
            8 => EventType::Resize,
            9 => EventType::PopupReponse,
            10 => EventType::MousePress,
            11 => EventType::MouseRelease,
            12 => EventType::Shutdown,
            _ => panic!("unkown request code"),
        }
    }
}
lazy_static! {
    static ref EVENT_ARGS_MAP: BTreeMap<EventType, Vec<EventArgType>> = BTreeMap::from([
        (
            EventType::NewWindow,
            vec![EventArgType::Id(0), EventArgType::Size(Size::zero())]
        ),
        (EventType::WindowClose, vec![]),
        (
            EventType::MouseClick,
            vec![EventArgType::Point(Point::zero())]
        ),
        (
            EventType::MouseDrag,
            vec![EventArgType::Point(Point::zero())]
        ),
        (EventType::MenuItem, vec![EventArgType::U32(0)]),
        (EventType::KeyDown, vec![EventArgType::U32(0)]),
        (EventType::HeatBeat, vec![]),
        (EventType::Resize, vec![EventArgType::Size(Size::zero())]),
        (
            EventType::PopupReponse,
            vec![EventArgType::String(String::new())]
        ),
        (
            EventType::MousePress,
            vec![EventArgType::Point(Point::zero())]
        ),
        (
            EventType::MouseRelease,
            vec![EventArgType::Point(Point::zero())]
        ),
        (EventType::Shutdown, vec![]),
    ]);
}

impl EventArgType {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            EventArgType::Id(id) => id.to_be_bytes().to_vec(),
            EventArgType::Point(point) => (((point.x as i16 as u32) << 16)
                | (point.y as i16 as u32 & 0xFFFF))
                .to_be_bytes()
                .to_vec(),
            EventArgType::U32(val) => val.to_be_bytes().to_vec(),
            EventArgType::Size(size) => (((size.width as u32) << 16) | (size.height as u32))
                .to_be_bytes()
                .to_vec(),
            EventArgType::String(string) => {
                let mut vec = (string.len() as u32).to_be_bytes().to_vec();
                vec.extend(string.as_bytes());
                vec
            }
        }
    }
    pub fn decode(raw_data: &[u8], arg_type: &EventArgType) -> EventArgType {
        match arg_type {
            EventArgType::String(_) => {
                let str_len = u32::from_be_bytes(raw_data[0..4].try_into().unwrap());
                EventArgType::String(str::from_utf8(&raw_data[4..]).unwrap().to_string())
            }
            _ => {
                let val = u32::from_be_bytes(raw_data.try_into().unwrap());
                match arg_type {
                    EventArgType::Id(_) => EventArgType::Id(val),
                    EventArgType::U32(_) => EventArgType::U32(val),
                    EventArgType::Point(_) => EventArgType::Point(Point::new(
                        ((val >> 16) & 0xFFFF) as i16 as i32,
                        (val & 0xFFFF) as i16 as i32,
                    )),
                    EventArgType::Size(_) => {
                        EventArgType::Size(Size::new((val >> 16) as u32, val as u16 as u32))
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
// a real request
#[derive(Debug)]
pub struct Event {
    pub event_type: EventType,
    pub args: Vec<EventArgType>,
}
impl Event {
    pub fn new(request_type: EventType, args: Vec<EventArgType>) -> Self {
        let type_list = EVENT_ARGS_MAP.get(&request_type).unwrap();
        assert!(type_list.len() == args.len());
        assert!(args
            .iter()
            .zip(type_list)
            .all(|(a, b)| discriminant(a) == discriminant(b)));
        Self {
            event_type: request_type,
            args,
        }
    }
    pub fn new_window(window_id: u32, window_size: Size) -> Self {
        Self {
            event_type: EventType::NewWindow,
            args: vec![EventArgType::Id(window_id), EventArgType::Size(window_size)],
        }
    }
    pub fn window_close() -> Self {
        Self {
            event_type: EventType::WindowClose,
            args: vec![],
        }
    }
    pub fn mouse_click(position: Point) -> Self {
        Self {
            event_type: EventType::MouseClick,
            args: vec![EventArgType::Point(position)],
        }
    }
    pub fn mouse_press(position: Point) -> Self {
        Self {
            event_type: EventType::MousePress,
            args: vec![EventArgType::Point(position)],
        }
    }
    pub fn mouse_release(position: Point) -> Self {
        Self {
            event_type: EventType::MouseRelease,
            args: vec![EventArgType::Point(position)],
        }
    }
    pub fn mouse_drag(shift: Point) -> Self {
        Self {
            event_type: EventType::MouseDrag,
            args: vec![EventArgType::Point(shift)],
        }
    }
    pub fn menu_item(item_id: u32) -> Self {
        Self {
            event_type: EventType::MenuItem,
            args: vec![EventArgType::U32(item_id)],
        }
    }
    pub fn key_down(key_code: u32) -> Self {
        Self {
            event_type: EventType::KeyDown,
            args: vec![EventArgType::U32(key_code)],
        }
    }
    pub fn heatbeat() -> Self {
        Self {
            event_type: EventType::HeatBeat,
            args: vec![],
        }
    }
    pub fn resize(new_size: Size) -> Self {
        Self {
            event_type: EventType::Resize,
            args: vec![EventArgType::Size(new_size)],
        }
    }
    pub fn popup_response(text: String) -> Self {
        Self {
            event_type: EventType::PopupReponse,
            args: vec![EventArgType::String(text)],
        }
    }
    pub fn shutdown() -> Self {
        Self {
            event_type: EventType::Shutdown,
            args: vec![],
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut raw_data: Vec<u8> = vec![0];
        raw_data.extend(self.event_type.encode().to_be_bytes());
        for arg in &self.args {
            raw_data.extend(arg.encode());
        }
        let len = raw_data.len();
        assert!(len < 256);
        raw_data[0] = len as u8;
        raw_data
    }
    pub fn decode(raw_data: &[u8]) -> Option<Self> {
        // println!("raw_data:{:?}", raw_data);
        let event_type = EventType::decode(u32::from_be_bytes(raw_data[1..5].try_into().unwrap()));
        let mut offset = 5;
        let args_list = EVENT_ARGS_MAP.get(&event_type).unwrap();
        let mut args_vec: Vec<EventArgType> = vec![];
        for arg_type in args_list {
            let arg = match arg_type {
                &EventArgType::String(_) => {
                    let str_len =
                        u32::from_be_bytes(raw_data[offset..offset + 4].try_into().unwrap());
                    let old_offset = offset;
                    offset += 4 + str_len as usize;
                    EventArgType::decode(&raw_data[old_offset..offset], arg_type)
                }
                _ => {
                    let v = raw_data[offset..offset + 4].try_into().unwrap();
                    offset += 4;
                    EventArgType::decode(v, arg_type)
                }
            };
            args_vec.push(arg);
        }
        Some(Self::new(event_type, args_vec))
    }
}
// use this to receive  request
pub struct EventReceiver {
    buffer: [u8; SOCKET_BUFFER_SIZE],
    socket_fd: usize,
    events: VecDeque<Event>,
}
impl EventReceiver {
    pub fn new(socket_fd: usize) -> Self {
        Self {
            buffer: [0u8; SOCKET_BUFFER_SIZE],
            socket_fd,
            events: VecDeque::new(),
        }
    }
    pub fn receive(&mut self) -> Option<Event> {
        // TODO specific socket implement byte stream
        // may read particial data in real socket
        let read_size = read(self.socket_fd, &mut self.buffer) as usize;
        // TODO read head, check length
        if read_size > 0 {
            let mut index = 0;
            while index < read_size {
                let len = self.buffer[index] as usize;
                let event = Event::decode(&self.buffer[index..index + len])
                    .expect("EventReceiver: wrong event format");
                self.events.push_back(event);
                index += len;
            }
        }
        self.events.pop_front()
    }
}
// use this to send request
pub struct EventSender {
    buffer: [u8; SOCKET_BUFFER_SIZE],
    socket_fd: usize,
    already_send: usize,
}
impl EventSender {
    pub fn new(socket_fd: usize) -> Self {
        Self {
            buffer: [0u8; SOCKET_BUFFER_SIZE],
            socket_fd,
            already_send: 0,
        }
    }
    pub fn send(&mut self, event: Event) {
        let raw_data = event.encode();
        if self.already_send + raw_data.len() >= SOCKET_BUFFER_SIZE {
            self.flush();
        }
        self.buffer[self.already_send..self.already_send + raw_data.len()]
            .copy_from_slice(&raw_data);
        self.already_send += raw_data.len();
    }
    pub fn flush(&mut self) {
        if self.already_send > 0 {
            let write_size = write(self.socket_fd, &self.buffer[0..self.already_send]);
            assert!(write_size as usize == self.already_send);
            self.already_send = 0;
        }
    }
}
