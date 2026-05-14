use crate::AppData;
use crate::actions::Action;
use crate::process::{spawn_program, spawn_shell};
use crate::protocol::river::wayland_client::Proxy;
use crate::river::{
    river_seat_v1::{Modifiers, RiverSeatV1},
    river_window_manager_v1::RiverWindowManagerV1,
    river_window_v1::Edges,
    river_window_v1::RiverWindowV1,
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use crate::wm::desktop::Desktop;
use crate::wm::task::Task;
use crate::wm::utils::Position;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use wayland_backend::client::ObjectId;
use wayland_client::QueueHandle;

use super::{LayerFocus, Output, PointerBinding, Seat, SeatOp, Window, XkbBinding};

impl Seat {
    pub fn new(proxy: RiverSeatV1, tx: Sender<Task>) -> Self {
        Self {
            proxy,
            queue_tx: tx,
            new: true,
            removed: false,
            focused: None,
            hovered: None,
            interacted: None,
            xkb_bindings: HashMap::new(),
            pointer_bindings: HashMap::new(),
            pending_action: Action::None,
            op: SeatOp::None,
            op_dx: 0,
            op_dy: 0,
            op_release: false,
            layer: None,
            layer_focus: LayerFocus::None,
        }
    }

    pub fn create_xkb_binding(
        &mut self,
        river_xkb: &RiverXkbBindingsV1,
        qh: &QueueHandle<AppData>,
        mods: Modifiers,
        keysym: u32,
        action: Action,
    ) {
        let proxy = river_xkb.get_xkb_binding(&self.proxy, keysym, mods, qh, self.proxy.id());
        proxy.enable();
        let binding = XkbBinding { proxy, action };
        self.xkb_bindings.insert(binding.proxy.id(), binding);
    }

    pub fn create_pointer_binding(
        &mut self,
        qh: &QueueHandle<AppData>,
        mods: Modifiers,
        button: u32,
        action: Action,
    ) {
        let proxy = self
            .proxy
            .get_pointer_binding(button, mods, qh, self.proxy.id());
        proxy.enable();
        let binding = PointerBinding { proxy, action };
        self.pointer_bindings.insert(binding.proxy.id(), binding);
    }

    // NOTE: this is the stuff that happens on keybinding
    pub fn do_action(
        &mut self,
        desktop: &mut Desktop,
        windows: &mut HashMap<RiverWindowV1, Window>,
        outputs: &HashMap<ObjectId, Output>,
        wm_proxy: &RiverWindowManagerV1,
        camera_pos: &mut Position,
    ) {
        match &self.pending_action {
            Action::None => {}
            Action::Pan => {
                self.pointer_pan(camera_pos);
            }
            // TODO: this is clearly bad
            Action::View { x, y } => {
                *camera_pos = Position { x: *x, y: *y };
            }
            Action::Spawn { program, args } => {
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                spawn_program(program, &arg_refs)
            }
            Action::SpawnShell { command } => spawn_shell(command),
            Action::Close => {
                if let Some(window_id) = self.focused.clone() {
                    self.queue_tx
                        .send(Task::CloseWindow { window_id })
                        .expect("couldn't closewindow");
                }
            }
            Action::CenterFocused => {
                if let Some(window_proxy) = self.focused.as_ref() {
                    let window = windows
                        .values_mut()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Focused window {window.proxy.id()} not found");
                    self.focus_window_camera(window, outputs, camera_pos)
                }
            }
            Action::Move => {
                if let (Some(window_proxy), SeatOp::None) = (self.hovered.as_ref(), &self.op) {
                    let window = windows
                        .values()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Hovered window {window.proxy.id()} not found");
                    self.pointer_move(window);
                }
            }
            Action::Resize => {
                if let (Some(window_proxy), SeatOp::None) = (self.hovered.as_ref(), &self.op) {
                    let window = windows
                        .values()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Hovered window {window.proxy.id()} not found");
                    self.pointer_resize(window, Edges::Bottom.union(Edges::Right));
                }
            }
            Action::ToggleMaximize => {
                if let Some(window_proxy) = self.focused.clone() {
                    self.queue_tx
                        .send(Task::MaximizeWindow {
                            window_id: window_proxy,
                        })
                        .expect("couldn't send togglemaximize");
                }
            }
            Action::PrevSlide => {
                let workspace = desktop.active_workspace_mut();
                workspace.prev_slide();
                let position = workspace.slides[workspace.active_slide].position;
                // TODO: replace coord with Position
                self.queue_tx
                    .send(Task::MoveCamera { position })
                    .expect("couldn't send prevslide");
                let slide = workspace.active_slide_mut();
                if !slide.windows.is_empty() {
                    self.focus_window(&slide.windows[slide.active_window]);
                }
            }
            Action::NextSlide => {
                let workspace = desktop.active_workspace_mut();
                workspace.next_slide();
                let position = workspace.slides[workspace.active_slide].position;
                // TODO: replace coord with Position
                self.queue_tx
                    .send(Task::MoveCamera { position })
                    .expect("couldn't send nextslide");
                let slide = workspace.active_slide_mut();
                if !slide.windows.is_empty() {
                    self.focus_window(&slide.windows[slide.active_window]);
                }
            }
            Action::MoveToNextSlide => {
                let workspace = desktop.active_workspace_mut();
                workspace.moveto_next_slide(windows);
                *camera_pos = workspace.slides[workspace.active_slide].position;
            }
            Action::MoveToPrevSlide => {
                let workspace = desktop.active_workspace_mut();
                workspace.moveto_prev_slide(windows);
                *camera_pos = workspace.slides[workspace.active_slide].position;
            }
            Action::PrevWindow => {
                let slide = desktop.active_workspace_mut().active_slide_mut();
                slide.prev_window();
                // TODO: add config option to remove this (keyboard focus on slide change)
                // TODO: idt I should do this weird check
                // TODO: Not sure if I need to do this for all seats - if I do, I need a new
                // FocusOnWindow task prolly
                if !slide.windows.is_empty() {
                    self.focus_window(&slide.windows[slide.active_window])
                }
                slide.rearrange();
            }
            Action::NextWindow => {
                let slide = desktop.active_workspace_mut().active_slide_mut();
                slide.next_window();
                // TODO: add config option to remove this (keyboard focus on slide change)
                // TODO: idt I should do this weird check
                // TODO: Not sure if I need to do this for all seats - if I do, I need a new
                // FocusOnWindow task prolly
                if !slide.windows.is_empty() {
                    self.focus_window(&slide.windows[slide.active_window])
                }
                slide.rearrange();
            }
            Action::CycleTiling => {
                desktop
                    .active_workspace_mut()
                    .active_slide_mut()
                    .cycle_tiling();
            }
            Action::Exit => wm_proxy.exit_session(),
        }
        self.pending_action = Action::None;
    }

    pub fn op_end(&mut self) {
        if let SeatOp::Resize { window_proxy, .. } = &self.op {
            window_proxy.inform_resize_end();
        }
        self.proxy.op_end();
        self.op = SeatOp::None;
    }

    pub fn op_manage(&mut self) {
        match &self.op {
            SeatOp::None | SeatOp::Move { .. } => {}
            SeatOp::Pan { .. } => {}
            SeatOp::Resize {
                window_proxy,
                start_width,
                start_height,
                edges,
                ..
            } => {
                let (mut width, mut height) = (*start_width, *start_height);
                if edges.contains(Edges::Left) {
                    width -= self.op_dx;
                }
                if edges.contains(Edges::Right) {
                    width += self.op_dx;
                }
                if edges.contains(Edges::Top) {
                    height -= self.op_dy;
                }
                if edges.contains(Edges::Bottom) {
                    height += self.op_dy;
                }
                window_proxy.propose_dimensions(width, height);
            }
        }
    }

    pub fn focus_window(&mut self, window_id: &RiverWindowV1) {
        match self.layer_focus {
            LayerFocus::Exclusive => {
                self.proxy.clear_focus();
                self.focused = None;
                return;
            }
            LayerFocus::NonExclusive => {}
            LayerFocus::None => {}
        }
        self.proxy.focus_window(window_id);
        self.focused = Some(window_id.clone());
    }

    // pub fn focus_top(&mut self, windows: &HashMap<RiverWindowV1, Window>, desktop: &mut Desktop) {
    //     match self.layer_focus {
    //         LayerFocus::Exclusive => {
    //             self.proxy.clear_focus();
    //             self.focused = None;
    //             return;
    //         }
    //         LayerFocus::NonExclusive => {}
    //         LayerFocus::None => {}
    //     }
    //
    //     let slide = desktop.active_workspace_mut().active_slide_mut();
    //
    //     match slide.windows.last_mut() {
    //         Some(window) => {
    //             self.proxy.focus_window(window); // for inputs
    //             // TODO: I really really should do this
    //             // window.node.place_top(); // render on top
    //             self.focused = Some(window.clone()); // for bookkeeping
    //         }
    //         None => {
    //             self.proxy.clear_focus();
    //             self.focused = None;
    //         }
    //     }
    // }

    // TODO: convert seatop to use camera_pos
    fn pointer_pan(&mut self, camera_pos: &mut Position) {
        self.proxy.op_start_pointer();
        self.op = SeatOp::Pan {
            start_x: camera_pos.x,
            start_y: camera_pos.y,
        };
        self.op_dx = 0;
        self.op_dy = 0;
    }

    pub fn pointer_move(&mut self, window: &Window) {
        self.interacted = Some(window.proxy.clone());
        self.proxy.op_start_pointer();
        self.op = SeatOp::Move {
            window_proxy: window.proxy.clone(),
            start_x: window.x,
            start_y: window.y,
        };
        self.op_dx = 0;
        self.op_dy = 0;
    }

    pub fn pointer_resize(&mut self, window: &Window, edges: Edges) {
        self.interacted = Some(window.proxy.clone());
        self.proxy.op_start_pointer();
        window.proxy.inform_resize_start();
        self.op = SeatOp::Resize {
            window_proxy: window.proxy.clone(),
            start_x: window.x,
            start_y: window.y,
            start_width: window.width,
            start_height: window.height,
            edges,
        };
        self.op_dx = 0;
        self.op_dy = 0;
    }

    fn focus_window_camera(
        &self,
        window: &Window,
        outputs: &HashMap<ObjectId, Output>,
        camera_pos: &mut Position,
    ) {
        let Some((screen_cx, screen_cy)) = outputs.values().find_map(|output| {
            let (x, y) = output.position?;
            let (w, h) = output.dimensions?;
            Some((x + w / 2, y + h / 2))
        }) else {
            return;
        };
        *camera_pos = Position {
            x: window.x + window.width / 2 - screen_cx,
            y: window.y + window.height / 2 - screen_cy,
        };
    }
}
