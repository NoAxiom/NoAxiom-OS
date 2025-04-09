use alloc::collections::{BTreeMap, VecDeque};

use super::*;
#[macro_use]
use alloc::vec;
use syscall::{read, write};
extern crate derive_more;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{convert::TryInto, mem::discriminant, str};

use derive_more::{From, Into, TryInto};
use embedded_graphics::{
    pixelcolor::{raw::RawU24, Rgb888},
    prelude::*,
};
use lazy_static::lazy_static;
const SOCKET_BUFFER_SIZE: usize = 256;

#[derive(Ord, Eq, PartialEq, PartialOrd, Debug)]
pub enum RequestType {
    NewWindow,
    NewFrame,
    DrawLine,
    DrawRect,
    DrawText,
    AddMenuItem,
    AddSubMenuItem,
    PopupRequest,
    PopupResponse,
    CloseWindow,
    Shutdown,
}
#[derive(Debug, Clone, TryInto)]
pub enum RequestArgType {
    Id(u32),
    Color(Rgb888),
    Point(Point),
    Size(Size),
    U32(u32),
    Bool(bool),
    String(String),
}
impl RequestType {
    pub fn encode(&self) -> u32 {
        match self {
            RequestType::NewWindow => 1,
            RequestType::NewFrame => 2,
            RequestType::DrawLine => 3,
            RequestType::DrawRect => 4,
            RequestType::DrawText => 5,
            RequestType::AddMenuItem => 6,
            RequestType::AddSubMenuItem => 7,
            RequestType::PopupRequest => 8,
            RequestType::PopupResponse => 9,
            RequestType::CloseWindow => 10,
            RequestType::Shutdown => 11,
        }
    }
    pub fn decode(code: u32) -> Self {
        match code {
            1 => RequestType::NewWindow,
            2 => RequestType::NewFrame,
            3 => RequestType::DrawLine,
            4 => RequestType::DrawRect,
            5 => RequestType::DrawText,
            6 => RequestType::AddMenuItem,
            7 => RequestType::AddSubMenuItem,
            8 => RequestType::PopupRequest,
            9 => RequestType::PopupResponse,
            10 => RequestType::CloseWindow,
            11 => RequestType::Shutdown,
            _ => panic!("unkown request code"),
        }
    }
}
lazy_static! {
    static ref REQUEST_ARGS_MAP: BTreeMap<RequestType, Vec<RequestArgType>> = BTreeMap::from([
        (
            RequestType::NewWindow,
            vec![
                RequestArgType::String(String::new()),
                RequestArgType::Bool(true)
            ]
        ),
        (RequestType::NewFrame, vec![]),
        (
            RequestType::DrawLine,
            vec![
                RequestArgType::Point(Point::zero()),
                RequestArgType::Point(Point::zero()),
                RequestArgType::U32(0),
                RequestArgType::Color(Rgb888::WHITE)
            ]
        ),
        (
            RequestType::DrawRect,
            vec![
                RequestArgType::Point(Point::zero()),
                RequestArgType::Size(Size::zero()),
                RequestArgType::U32(0),
                RequestArgType::Color(Rgb888::WHITE),
                RequestArgType::Color(Rgb888::WHITE)
            ]
        ),
        (
            RequestType::DrawText,
            vec![
                RequestArgType::String(String::new()),
                RequestArgType::Point(Point::zero()),
                RequestArgType::Bool(true),
                RequestArgType::Color(Rgb888::WHITE)
            ]
        ),
        (
            RequestType::AddMenuItem,
            vec![RequestArgType::String(String::new()),]
        ),
        (
            RequestType::AddSubMenuItem,
            vec![
                RequestArgType::U32(0),
                RequestArgType::String(String::new()),
            ]
        ),
        (
            RequestType::PopupRequest,
            vec![RequestArgType::String(String::new()),]
        ),
        (
            RequestType::PopupResponse,
            vec![RequestArgType::String(String::new()),]
        ),
        (RequestType::CloseWindow, vec![]),
        (RequestType::Shutdown, vec![]),
    ]);
}

impl RequestArgType {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            RequestArgType::Id(id) => id.to_be_bytes().to_vec(),
            RequestArgType::Color(color) => color.into_storage().to_be_bytes().to_vec(),
            RequestArgType::Point(point) => (((point.x as i16 as u32) << 16)
                | (point.y as i16 as u32) & 0xFFFF)
                .to_be_bytes()
                .to_vec(),
            RequestArgType::Size(size) => (((size.width as u32) << 16) | (size.height as u32))
                .to_be_bytes()
                .to_vec(),
            RequestArgType::U32(val) => val.to_be_bytes().to_vec(),
            RequestArgType::Bool(val) => (if val.clone() { 1u32 } else { 0u32 })
                .to_be_bytes()
                .to_vec(),
            RequestArgType::String(string) => {
                let mut vec = (string.len() as u32).to_be_bytes().to_vec();
                vec.extend(string.as_bytes());
                vec
            }
        }
    }
    pub fn decode(raw_data: &[u8], arg_type: &RequestArgType) -> RequestArgType {
        match arg_type {
            // variable len
            RequestArgType::String(_) => {
                let str_len = u32::from_be_bytes(raw_data[0..4].try_into().unwrap());
                RequestArgType::String(str::from_utf8(&raw_data[4..]).unwrap().to_string())
            }
            // fixed len 4
            _ => {
                let val = u32::from_be_bytes(raw_data.try_into().unwrap());
                match arg_type {
                    RequestArgType::Id(_) => RequestArgType::Id(val),
                    RequestArgType::Color(_) => RequestArgType::Color(RawU24::new(val).into()),
                    RequestArgType::Point(_) => RequestArgType::Point(Point::new(
                        ((val >> 16) & 0xFFFF) as i16 as i32,
                        (val & 0xFFFF) as i16 as i32,
                    )),
                    RequestArgType::Size(_) => {
                        RequestArgType::Size(Size::new((val >> 16) as u32, val as u16 as u32))
                    }
                    RequestArgType::U32(_) => RequestArgType::U32(val),
                    RequestArgType::Bool(_) => {
                        RequestArgType::Bool(if val == 0 { false } else { true })
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
// a real request
#[derive(Debug)]
pub struct Request {
    pub request_type: RequestType,
    pub args: Vec<RequestArgType>,
}
impl Request {
    pub fn new(request_type: RequestType, args: Vec<RequestArgType>) -> Self {
        let type_list = REQUEST_ARGS_MAP.get(&request_type).unwrap();
        assert!(type_list.len() == args.len());
        assert!(args
            .iter()
            .zip(type_list)
            .all(|(a, b)| discriminant(a) == discriminant(b)));
        Self { request_type, args }
    }
    pub fn new_window(title: String, have_menubar: bool) -> Self {
        Self {
            request_type: RequestType::NewWindow,
            args: vec![
                RequestArgType::String(title),
                RequestArgType::Bool(have_menubar),
            ],
        }
    }
    pub fn new_frame() -> Self {
        Self {
            request_type: RequestType::NewFrame,
            args: vec![],
        }
    }
    pub fn draw_line(start: Point, end: Point, stoke_width: u32, color: Rgb888) -> Self {
        let args: Vec<RequestArgType> = vec![
            RequestArgType::Point(start),
            RequestArgType::Point(end),
            RequestArgType::U32(stoke_width),
            RequestArgType::Color(color),
        ];
        Self {
            request_type: RequestType::DrawLine,
            args,
        }
    }
    pub fn draw_rect(
        start: Point,
        size: Size,
        stoke_width: u32,
        stoke_color: Rgb888,
        fill_color: Rgb888,
    ) -> Self {
        let args: Vec<RequestArgType> = vec![
            RequestArgType::Point(start),
            RequestArgType::Size(size),
            RequestArgType::U32(stoke_width),
            RequestArgType::Color(stoke_color),
            RequestArgType::Color(fill_color),
        ];
        Self {
            request_type: RequestType::DrawRect,
            args,
        }
    }
    pub fn draw_text(text: String, position: Point, centered: bool, stoke_color: Rgb888) -> Self {
        let args: Vec<RequestArgType> = vec![
            RequestArgType::String(text),
            RequestArgType::Point(position),
            RequestArgType::Bool(centered),
            RequestArgType::Color(stoke_color),
        ];
        Self {
            request_type: RequestType::DrawText,
            args,
        }
    }
    pub fn add_menu_item(text: String) -> Self {
        let args: Vec<RequestArgType> = vec![RequestArgType::String(text)];
        Self {
            request_type: RequestType::AddMenuItem,
            args,
        }
    }
    pub fn add_sub_menu_item(main_item: u32, text: String) -> Self {
        let args: Vec<RequestArgType> =
            vec![RequestArgType::U32(main_item), RequestArgType::String(text)];
        Self {
            request_type: RequestType::AddSubMenuItem,
            args,
        }
    }
    pub fn popupp_request(text: String) -> Self {
        let args: Vec<RequestArgType> = vec![RequestArgType::String(text)];
        Self {
            request_type: RequestType::PopupRequest,
            args,
        }
    }
    pub fn popupp_response(text: String) -> Self {
        let args: Vec<RequestArgType> = vec![RequestArgType::String(text)];
        Self {
            request_type: RequestType::PopupResponse,
            args,
        }
    }
    pub fn close_window() -> Self {
        Self {
            request_type: RequestType::CloseWindow,
            args: vec![],
        }
    }
    pub fn shutdown() -> Self {
        Self {
            request_type: RequestType::Shutdown,
            args: vec![],
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut raw_data: Vec<u8> = vec![0];
        raw_data.extend(self.request_type.encode().to_be_bytes());
        for arg in &self.args {
            raw_data.extend(arg.encode());
        }
        let len = raw_data.len();
        assert!(len < 256);
        raw_data[0] = len as u8;
        raw_data
    }
    pub fn decode(raw_data: &[u8]) -> Option<Self> {
        println!("raw_data:{:?}", raw_data);
        let request_type =
            RequestType::decode(u32::from_be_bytes(raw_data[1..5].try_into().unwrap()));
        let mut offset = 5;
        let args_list = REQUEST_ARGS_MAP.get(&request_type).unwrap();
        let mut args_vec: Vec<RequestArgType> = vec![];
        for arg_type in args_list {
            let arg = match arg_type {
                RequestArgType::String(_) => {
                    let str_len =
                        u32::from_be_bytes(raw_data[offset..offset + 4].try_into().unwrap());
                    let old_offset = offset;
                    offset += 4 + str_len as usize;
                    RequestArgType::decode(&raw_data[old_offset..offset], arg_type)
                }
                _ => {
                    let v = raw_data[offset..offset + 4].try_into().unwrap();
                    offset += 4;
                    RequestArgType::decode(v, arg_type)
                }
            };
            args_vec.push(arg);
        }
        Some(Self::new(request_type, args_vec))
    }
}
// use this to receive  request
pub struct RequestReceiver {
    buffer: [u8; SOCKET_BUFFER_SIZE],
    socket_fd: usize,
    requests: VecDeque<Request>,
}
impl RequestReceiver {
    pub fn new(socket_fd: usize) -> Self {
        Self {
            buffer: [0u8; SOCKET_BUFFER_SIZE],
            socket_fd,
            requests: VecDeque::new(),
        }
    }
    pub fn receive(&mut self) -> Option<Request> {
        // TODO specific socket implement byte stream
        // may read particial data in real socket
        let read_size = read(self.socket_fd, &mut self.buffer) as usize;
        // TODO read head, check length

        if read_size > 0 {        
            println!("read size {:?}",read_size);
            let mut index = 0;
            while index < read_size {
                let len = self.buffer[index] as usize;
                println!("len {:?}",len);
                let event = Request::decode(&self.buffer[index..index + len])
                    .expect("RequestReceiver: wrong request format");
                self.requests.push_back(event);
                index += len;
            }
        }
        self.requests.pop_front()
    }
}
// use this to send request
pub struct RequestSender {
    buffer: [u8; SOCKET_BUFFER_SIZE],
    socket_fd: usize,
    already_send: usize,
}
impl RequestSender {
    pub fn new(socket_fd: usize) -> Self {
        Self {
            buffer: [0u8; SOCKET_BUFFER_SIZE],
            socket_fd,
            already_send: 0,
        }
    }
    pub fn send(&mut self, request: Request) {
        let raw_data = request.encode();
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
