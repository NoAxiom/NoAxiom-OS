use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
};
use core::{alloc::Layout, borrow::Borrow, cell::RefCell, cmp::max, convert::TryInto, str};

use syscall::{close, connect, exit};

use super::*;
#[macro_use]
use alloc::format;
use alloc::{
    string::{String, ToString},
    vec::{self, Vec},
};

use embedded_graphics::{pixelcolor::Rgb888, prelude::*};

use self::{
    lib_event::{Event, EventArgType, EventReceiver, EventType},
    lib_request::{self, Request, RequestSender},
    lib_widget::{LayoutDirection, State, Widget, WidgetBox},
};
// use user::{
//     close, connect, event_get, framebuffer, get_time, listen, open, read,
// sleep, write, Mouse,     OpenFlags,
// };
type MenuHandler =
    fn(&mut VecDeque<Request>, &mut Option<String>, &mut VecDeque<String>, &mut State);
type PopupHandler =
    fn(String, &mut VecDeque<Request>, &mut Option<String>, &mut VecDeque<String>, &mut State);
const SERVER_NAME: &str = "server\0";
pub struct Client {
    request_sender: RequestSender,
    event_receiver: EventReceiver,
    widget_map: BTreeMap<usize, RefCell<Box<dyn Widget>>>,
    root_widget: Option<usize>,
    pending_requests: VecDeque<Request>,
    keyboard_focus: usize,
    size: Size,
    id_counter: usize,
    pub need_update: bool,
    state: State,
    popup_request: Option<String>,
    pub closed: bool,
    action_map: BTreeMap<String, usize>,
    pub pending_actions: VecDeque<String>,
    pub press_on: Option<usize>,
    menu_handler: BTreeMap<usize, MenuHandler>,
    popup_handler: BTreeMap<String, PopupHandler>,
}
impl Client {
    pub fn init(title: String, have_menubar: bool) -> Self {
        let fd = loop {
            let fd = connect(SERVER_NAME);
            if fd > 0 {
                break fd as usize;
            }
        };
        let mut request_sender = RequestSender::new(fd);
        let mut event_receiver = EventReceiver::new(fd);
        let request = Request::new_window(title, have_menubar);
        request_sender.send(request);
        request_sender.flush();
        let (window_id, size) = loop {
            if let Some(event) = event_receiver.receive() {
                match event.event_type {
                    EventType::NewWindow => {
                        let id: u32 = event.args[0].clone().try_into().unwrap();
                        let size: Size = event.args[1].clone().try_into().unwrap();
                        break (id as usize, size);
                    }
                    _ => panic!("wrong event type"),
                }
            }
        };
        close(fd); // untest for socket close
        let client_name = format!("CLIENT_{:0>3}\0", window_id);
        let fd = connect(client_name.as_str()) as usize;
        Self {
            request_sender: RequestSender::new(fd),
            event_receiver: EventReceiver::new(fd),
            root_widget: None,
            widget_map: BTreeMap::new(),
            pending_requests: VecDeque::new(),
            keyboard_focus: 0,
            id_counter: 1,
            size,
            need_update: true,
            state: State {
                data: BTreeMap::new(),
            },
            popup_request: None,
            closed: false,
            action_map: BTreeMap::new(),
            pending_actions: VecDeque::new(),
            press_on: None,
            menu_handler: BTreeMap::new(),
            popup_handler: BTreeMap::new(),
        }
    }
    pub fn receive_event(&mut self) -> Option<Event> {
        self.event_receiver.receive()
    }
    pub fn add_menu_item(&mut self, name: String) {
        let request = Request::add_menu_item(name);
        self.pending_requests.push_back(request);
    }
    pub fn add_sub_menu_item(&mut self, index: u32, name: String) {
        let request = Request::add_sub_menu_item(index, name);
        self.pending_requests.push_back(request);
    }
    pub fn attach_root_widget(&mut self, mut widget: Box<dyn Widget>) -> usize {
        let id = self.get_new_id();
        widget.set_id(id);
        self.widget_map.insert(id, RefCell::new(widget));
        self.root_widget = Some(id);
        self.resize_widgets();
        id
    }
    pub fn add_child_widget(&mut self, parent_id: usize, mut widget: Box<dyn Widget>) -> usize {
        let id = self.get_new_id();
        widget.set_id(id);
        self.widget_map.insert(id, RefCell::new(widget));
        {
            let mut parent_widget = self.widget_map.get(&parent_id).unwrap().borrow_mut();
            if let Some(widget_box) = parent_widget.as_any_mut().downcast_mut::<WidgetBox>() {
                widget_box.add_child(id);
            } else {
                panic!("add child to non box widget");
            };
        }
        self.resize_widgets();
        id
    }
    pub fn draw(&mut self) {
        fn draw_widget_subtree(
            client: &Client,
            widget_id: usize,
            pending_requests: &mut VecDeque<Request>,
            client_state: &State,
        ) {
            let widget = client.widget_map.get(&widget_id).unwrap().borrow();
            widget.draw(pending_requests, client_state);
            if let Some(widget_box) = widget.as_any().downcast_ref::<WidgetBox>() {
                for child_node in widget_box.get_children() {
                    draw_widget_subtree(client, child_node.clone(), pending_requests, client_state);
                }
            }
        }

        self.need_update = false;
        let mut pending_requests: VecDeque<Request> = VecDeque::new();
        if let Some(widget_id) = &self.root_widget {
            draw_widget_subtree(self, widget_id.clone(), &mut pending_requests, &self.state)
        }
        self.pending_requests.append(&mut pending_requests);
        self.pending_requests.push_back(Request::new_frame());
    }
    pub fn resize(&mut self, new_size: Size) {
        self.size = new_size;
        self.resize_widgets()
    }
    pub fn resize_widgets(&mut self) {
        fn resize_widget_subtree(
            widget_map: &BTreeMap<usize, RefCell<Box<dyn Widget>>>,
            widget_id: usize,
        ) {
            let widget_id = widget_id;
            let widget = widget_map.get(&widget_id).unwrap().borrow();
            let widget_size = widget.get_size();
            let widget_position = widget.get_position();
            // box widget
            if let Some(widget_box) = widget.as_any().downcast_ref::<WidgetBox>() {
                let mut flex_count = 0;
                match widget_box.layout {
                    LayoutDirection::Vertical => {
                        let mut total_height = widget_size.height;
                        for child_widget_id in widget_box.get_children() {
                            let child_widget = widget_map.get(&child_widget_id).unwrap().borrow();
                            let (_, y_flex) = child_widget.is_flex();
                            if y_flex {
                                flex_count += 1;
                            } else {
                                total_height -= child_widget.get_size().height
                                    + 2 * child_widget.get_margin() as u32;
                            }
                        }
                        total_height = max(0, total_height);
                        let averge_height = total_height / max(flex_count, 1);
                        // set size
                        for child_widget_id in widget_box.get_children() {
                            let mut child_widget =
                                widget_map.get(&child_widget_id).unwrap().borrow_mut();
                            let (x_flex, y_flex) = child_widget.is_flex();
                            let child_size = child_widget.get_size();
                            let child_margin = child_widget.get_margin();
                            let new_x: u32 = if x_flex {
                                widget_size.width - 2 * child_margin as u32
                            } else {
                                child_size.width
                            };
                            let new_y = if y_flex {
                                averge_height - 2 * child_margin as u32
                            } else {
                                child_size.height
                            };
                            child_widget.set_size(Size::new(new_x, new_y));
                        }
                        // set position
                        let mut position = widget_position;
                        for child_widget_id in widget_box.get_children() {
                            let mut child_widget =
                                widget_map.get(&child_widget_id).unwrap().borrow_mut();
                            let child_size = child_widget.get_size();
                            let child_margin = child_widget.get_margin();
                            let child_margin_size =
                                Size::new(child_margin as u32, child_margin as u32);
                            let (x_flex, _) = child_widget.is_flex();
                            let child_position = if x_flex {
                                position + child_margin_size
                            } else {
                                // center
                                let space = max(0, widget_size.width - child_size.width);
                                position + child_margin_size.y_axis() + Size::new(space / 2, 0)
                            };

                            child_widget.set_position(child_position);
                            position += child_size.y_axis() + child_margin_size.y_axis() * 2;
                        }
                        // resize children
                        for child_widget_id in widget_box.get_children() {
                            resize_widget_subtree(&widget_map, child_widget_id.clone());
                        }
                    }
                    LayoutDirection::Horizontal => {
                        let mut total_width = widget_size.width;
                        for child_widget_id in widget_box.get_children() {
                            let child_widget = widget_map.get(&child_widget_id).unwrap().borrow();
                            let (x_flex, _) = child_widget.is_flex();
                            if x_flex {
                                flex_count += 1;
                            } else {
                                total_width -= child_widget.get_size().width
                                    + 2 * child_widget.get_margin() as u32;
                            }
                        }
                        total_width = max(0, total_width);
                        let averge_width = total_width / max(flex_count, 1);
                        // set size
                        for child_widget_id in widget_box.get_children() {
                            let mut child_widget =
                                widget_map.get(&child_widget_id).unwrap().borrow_mut();
                            let (x_flex, y_flex) = child_widget.is_flex();
                            let child_size = child_widget.get_size();
                            let child_margin = child_widget.get_margin();
                            let new_x = if x_flex {
                                averge_width - 2 * child_margin as u32
                            } else {
                                child_size.width
                            };
                            let new_y: u32 = if y_flex {
                                widget_size.height - 2 * child_margin as u32
                            } else {
                                child_size.height
                            };
                            child_widget.set_size(Size::new(new_x, new_y));
                        }
                        // set position
                        let mut position = widget_position;
                        for child_widget_id in widget_box.get_children() {
                            let mut child_widget =
                                widget_map.get(&child_widget_id).unwrap().borrow_mut();
                            let child_size = child_widget.get_size();
                            let child_margin = child_widget.get_margin();
                            let child_margin_size =
                                Size::new(child_margin as u32, child_margin as u32);
                            let (_, y_flex) = child_widget.is_flex();
                            let child_position = if y_flex {
                                position + child_margin_size
                            } else {
                                // center
                                let space = max(0, widget_size.height - child_size.height);
                                position + child_margin_size.x_axis() + Size::new(0, space / 2)
                            };

                            child_widget.set_position(child_position);
                            position += child_size.x_axis() + child_margin_size.x_axis() * 2;
                        }
                        // resize children
                        for child_widget_id in widget_box.get_children() {
                            resize_widget_subtree(&widget_map, child_widget_id.clone());
                        }
                    }
                }
            }
        }
        let new_size = self.size;
        if let Some(widget_id) = &self.root_widget {
            {
                let mut widget = self.widget_map.get(&widget_id).unwrap().borrow_mut();
                widget.set_size(new_size);
            }
            resize_widget_subtree(&self.widget_map, widget_id.clone())
        }
    }
    pub fn flush(&mut self) {
        for request in self.pending_requests.drain(..) {
            match request.request_type {
                lib_request::RequestType::CloseWindow => self.closed = true,
                _ => {}
            }
            self.request_sender.send(request);
        }
        self.request_sender.flush();
    }
    pub fn send_request(&mut self, request: Request) {
        self.pending_requests.push_back(request);
    }
    pub fn register_action(&mut self, action: String, widget_id: usize) {
        self.action_map.insert(action, widget_id);
    }
    pub fn get_new_id(&mut self) -> usize {
        self.id_counter += 1;
        self.id_counter - 1
    }
    pub fn find_widget_by_position(
        &self,
        position: Point,
        handle_mouse: bool,
        handle_keyboard: bool,
    ) -> Option<usize> {
        fn point_in_rec(p: Point, position: Point, size: Size) -> bool {
            p.x >= position.x
                && p.y >= position.y
                && p.x < (position + size).x
                && p.y < (position + size).y
        }
        for (widget_id, widget) in &self.widget_map {
            let widget = widget.borrow();
            if handle_keyboard && !widget.can_have_keyboard_events() {
                continue;
            }
            if handle_mouse && !widget.can_have_mouse_events() {
                continue;
            }
            if point_in_rec(position, widget.get_position(), widget.get_size()) {
                return Some(widget_id.clone());
            }
        }
        None
    }
    pub fn register_menu_handler(&mut self, item_id: usize, handler: MenuHandler) {
        self.menu_handler.insert(item_id, handler);
    }
    pub fn register_popup_handler(&mut self, popup_type: String, handler: PopupHandler) {
        self.popup_handler.insert(popup_type, handler);
    }

    pub fn on_event(&mut self) {
        loop {
            match self.event_receiver.receive() {
                Some(event) => {
                    self.need_update = true;
                    println!("client event:{:?}", event);
                    match event.event_type {
                        EventType::Resize => {
                            let new_size: Size = event.args[0].clone().try_into().unwrap();
                            self.resize(new_size);
                        }
                        EventType::MouseClick => {
                            let clic_position = event.args[0].clone().try_into().unwrap();
                            if let Some(widget_id) =
                                self.find_widget_by_position(clic_position, true, false)
                            {
                                let mut widget =
                                    self.widget_map.get(&widget_id).unwrap().borrow_mut();
                                if widget.can_have_keyboard_events() {
                                    self.keyboard_focus = widget_id;
                                }
                                widget.on_event(
                                    event,
                                    &mut self.pending_requests,
                                    &mut self.pending_actions,
                                    &mut self.state,
                                );
                            }
                        }
                        EventType::KeyDown => {
                            if let Some(widget) = self.widget_map.get(&self.keyboard_focus) {
                                let mut widget = widget.borrow_mut();
                                widget.on_event(
                                    event,
                                    &mut self.pending_requests,
                                    &mut self.pending_actions,
                                    &mut self.state,
                                );
                            }
                        }
                        EventType::MenuItem => {
                            let item_id: u32 = event.args[0].clone().try_into().unwrap();
                            if let Some(handler) = self.menu_handler.get(&(item_id as usize)) {
                                handler(
                                    &mut self.pending_requests,
                                    &mut self.popup_request,
                                    &mut self.pending_actions,
                                    &mut self.state,
                                )
                            }
                        }
                        EventType::PopupReponse => {
                            let result: String = event.args[0].clone().try_into().unwrap();
                            if let Some(popup_request) = &self.popup_request {
                                if let Some(handler) = self.popup_handler.get(popup_request) {
                                    handler(
                                        result,
                                        &mut self.pending_requests,
                                        &mut self.popup_request,
                                        &mut self.pending_actions,
                                        &mut self.state,
                                    )
                                }
                            }
                        }
                        EventType::MousePress => {
                            let press_position = event.args[0].clone().try_into().unwrap();
                            if let Some(widget_id) =
                                self.find_widget_by_position(press_position, true, false)
                            {
                                self.press_on = Some(widget_id);
                                let mut widget =
                                    self.widget_map.get(&widget_id).unwrap().borrow_mut();
                                if widget.can_have_keyboard_events() {
                                    self.keyboard_focus = widget_id;
                                }
                                widget.on_event(
                                    event,
                                    &mut self.pending_requests,
                                    &mut self.pending_actions,
                                    &mut self.state,
                                );
                            }
                        }
                        EventType::MouseDrag => {
                            if let Some(widget_id) = self.press_on {
                                let mut widget =
                                    self.widget_map.get(&widget_id).unwrap().borrow_mut();
                                widget.on_event(
                                    event,
                                    &mut self.pending_requests,
                                    &mut self.pending_actions,
                                    &mut self.state,
                                );
                            }
                        }
                        EventType::MouseRelease => {
                            if let Some(widget_id) = self.press_on.take() {
                                let mut widget =
                                    self.widget_map.get(&widget_id).unwrap().borrow_mut();
                                widget.on_event(
                                    event,
                                    &mut self.pending_requests,
                                    &mut self.pending_actions,
                                    &mut self.state,
                                );
                            }
                        }
                        EventType::Shutdown => {
                            exit(0);
                        }
                        _ => {}
                    }
                }
                None => break,
            }
        }
    }
    pub fn on_action(&mut self) {
        for action in self.pending_actions.drain(..) {
            println!("a action:{:?}", action);
            if let Some(widget_id) = self.action_map.get(&action) {
                println!("widget_id:{:?}", widget_id);
                let mut widget = self.widget_map.get(&widget_id).unwrap().borrow_mut();
                widget.on_action(action, &mut self.state);
            }
        }
    }
}
