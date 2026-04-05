use super::{SeatOp, WindowManager};
use crate::AppData;
use crate::actions::{Action, parse_action, parse_keysym, parse_modifiers};
use crate::config::{Config, WindowConfig};
use crate::protocol::river::wayland_client::Proxy;
use crate::river::{
    river_seat_v1::Modifiers, river_window_manager_v1::RiverWindowManagerV1,
    river_window_v1::Edges, river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use crate::wm::Desktop;
use crate::wm::ObjectId;
use crate::wm::slide::Slide;
use crate::wm::workspace::Workspace;
use std::collections::{HashMap, VecDeque};
use wayland_client::QueueHandle;

impl WindowManager {
    pub fn handle_manage_start(
        &mut self,
        proxy: &RiverWindowManagerV1,
        river_xkb: &RiverXkbBindingsV1,
        qh: &QueueHandle<AppData>,
        config: &Config,
    ) {
        self.remove_outputs();
        self.remove_windows();
        self.remove_seats();
        self.init_new_windows(&config.window);
        self.init_new_seats(river_xkb, qh, config);
        self.manage_windows();
        self.manage_seats(proxy);
        proxy.manage_finish();
    }

    pub fn handle_render_start(&mut self, proxy: &RiverWindowManagerV1) {
        for seat in self.seats.values_mut() {
            match &seat.op {
                SeatOp::None => {}
                SeatOp::Pan { start_x, start_y } => {
                    self.camera_x = start_x - seat.op_dx;
                    self.camera_y = start_y - seat.op_dy;
                }
                SeatOp::Move {
                    window_proxy,
                    start_x,
                    start_y,
                } => {
                    if let Some(window) = self
                        .windows
                        .iter_mut()
                        .find(|window| &window.proxy == window_proxy)
                    {
                        window.set_position(start_x + seat.op_dx, start_y + seat.op_dy);
                    }
                }
                SeatOp::Resize {
                    window_proxy,
                    start_x,
                    start_y,
                    start_width,
                    start_height,
                    edges,
                } => {
                    if let Some(window) = self
                        .windows
                        .iter_mut()
                        .find(|window| &window.proxy == window_proxy)
                    {
                        let (mut x, mut y) = (*start_x, *start_y);
                        if edges.contains(Edges::Left) {
                            x += start_width - window.width;
                        }
                        if edges.contains(Edges::Top) {
                            y += start_height - window.height;
                        }
                        window.set_position(x, y);
                    }
                }
            }
        }

        let center = self
            .outputs
            .values()
            .find_map(|o| o.usable)
            .map(|(x, y, w, h)| (x + w / 2, y + h / 2));

        for window in self.windows.iter_mut() {
            if let Some(false) = window.hidden {
                if let Some((cx, cy)) = center {
                    window.set_position(
                        cx + self.camera_x - window.width / 2,
                        cy + self.camera_y - window.height / 2,
                    );
                }
                window.proxy.show();
                window.hidden = None;
                window.new = false
            }
            window.set_node_position(self.camera_x, self.camera_y);
        }

        proxy.render_finish();
    }

    pub fn remove_outputs(&mut self) {
        self.outputs.retain(|_, output| {
            if output.removed {
                if let Some(layer) = output.layer.take() {
                    layer.destroy();
                }
                output.proxy.destroy();
                return false;
            }
            true
        });
    }

    pub fn remove_windows(&mut self) {
        let old_windows = std::mem::take(&mut self.windows);
        self.windows = old_windows
            .into_iter()
            .filter(|window| {
                if window.closed {
                    for seat in self.seats.values_mut() {
                        if let SeatOp::Move { window_proxy, .. }
                        | SeatOp::Resize { window_proxy, .. } = &seat.op
                            && window_proxy == &window.proxy
                        {
                            seat.op_end();
                        }
                    }
                    return false;
                }
                true
            })
            .collect();
    }

    pub fn remove_seats(&mut self) {
        self.seats.retain(|_, seat| {
            if seat.removed {
                if let Some(layer) = seat.layer.take() {
                    layer.destroy();
                }
                seat.xkb_bindings
                    .values_mut()
                    .for_each(|binding| binding.proxy.destroy());
                seat.pointer_bindings
                    .values_mut()
                    .for_each(|binding| binding.proxy.destroy());
                seat.proxy.destroy();
                return false;
            }
            true
        });
    }

    pub fn init_new_windows(&mut self, window_config: &WindowConfig) {
        for window in self.windows.iter_mut().filter(|w| w.new) {
            window.proxy.propose_dimensions(window.width, window.height);
            if window_config.force_ssd {
                window.proxy.use_ssd();
            }
            window.proxy.hide();
        }
    }

    pub fn init_new_seats(
        &mut self,
        river_xkb: &RiverXkbBindingsV1,
        qh: &QueueHandle<AppData>,
        config: &Config,
    ) {
        for seat in self.seats.values_mut().filter(|seat| seat.new) {
            for (mods_name, keymap) in &config.bindings {
                let Some(mods) = parse_modifiers(mods_name) else {
                    eprintln!("Unknown modifier group: {mods_name}");
                    continue;
                };

                for (key_name, action_text) in keymap {
                    let Some(keysym) = parse_keysym(key_name) else {
                        eprintln!("Unknown key: {key_name}");
                        continue;
                    };
                    let Some(action) = parse_action(action_text) else {
                        eprintln!("Unknown action: {action_text}");
                        continue;
                    };

                    seat.create_xkb_binding(river_xkb, qh, mods, keysym, action);
                }
            }

            const BTN_LEFT: u32 = 0x110;
            const BTN_RIGHT: u32 = 0x111;
            const BTN_MIDDLE: u32 = 0x112;
            let mods = Modifiers::Mod1;
            seat.create_pointer_binding(qh, mods, BTN_LEFT, Action::Move);
            seat.create_pointer_binding(qh, mods, BTN_RIGHT, Action::Resize);
            seat.create_pointer_binding(qh, Modifiers::None, BTN_MIDDLE, Action::Pan);

            seat.new = false;
        }
    }

    pub fn manage_windows(&mut self) {
        for window in self.windows.iter_mut() {
            if let Some(seat_proxy) = window.pointer_move_requested.take() {
                let seat = self
                    .seats
                    .get_mut(&seat_proxy.id())
                    .unwrap_or_else(|| panic!("Seat {} not found", seat_proxy.id()));
                // TODO: remove this pattern
                // .expect("Seat {seat_proxy.id()} not found");
                seat.pointer_move(window);
            }
            if let Some(seat_proxy) = window.pointer_resize_requested.take() {
                let seat = self
                    .seats
                    .get_mut(&seat_proxy.id())
                    .expect("Seat {seat_proxy.id()} not found");
                seat.pointer_resize(window, window.pointer_resize_requested_edges);
            }
            if let Some(maximize) = window.maximize_requested.take() {
                if maximize {
                    let Some((width, height)) = self.outputs.values().find_map(|output| {
                        let (width, height) = output.dimensions?;
                        Some((width, height))
                    }) else {
                        continue;
                    };
                    window.unmaximized_geometry =
                        Some((window.x, window.y, window.width, window.height));
                    window.set_position(self.camera_x, self.camera_y);
                    window.width = width;
                    window.height = height;
                    window.proxy.propose_dimensions(width, height);
                    window.proxy.inform_maximized();
                } else {
                    if let Some((x, y, w, h)) = window.unmaximized_geometry.take() {
                        window.set_position(x, y);
                        window.width = w;
                        window.height = h;
                        window.proxy.propose_dimensions(w, h);
                        window.proxy.inform_unmaximized();
                    }
                }
            }
        }
    }

    pub fn manage_seats(&mut self, wm_proxy: &RiverWindowManagerV1) {
        let windows = &mut self.windows;
        let camera_x = &mut self.camera_x;
        let camera_y = &mut self.camera_y;

        for seat in self.seats.values_mut() {
            if let Some(window_proxy) = seat.interacted.take() {
                let i = windows
                    .iter()
                    .position(|window| window.proxy == window_proxy)
                    .expect("Interacted window {window.proxy.id()} not found");
                let window = windows.remove(i).unwrap();
                windows.push_back(window);
            }
            seat.focus_top(windows);
            seat.do_action(windows, &self.outputs, wm_proxy, camera_x, camera_y);
            if seat.op_release {
                seat.op_end();
                seat.op_release = false;
            } else {
                seat.op_manage();
            }
        }
    }
    pub fn active_workspace(&self) -> &Workspace {
        self.desktop
            .workspaces
            .get(self.desktop.active_workspace)
            .expect("active workspace index out of range")
    }
    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        let idx = self.desktop.active_workspace;
        self.desktop
            .workspaces
            .get_mut(idx)
            .expect("active workspace index out of range")
    }
    pub fn active_slide(&self) -> &Slide {
        let workspace = self.active_workspace();
        workspace
            .slides
            .get(workspace.focused_slide)
            .expect("active slide index out of range")
    }
    pub fn active_slide_mut(&mut self) -> &mut Slide {
        let workspace_idx = self.desktop.active_workspace;
        let slide_idx = self.desktop.workspaces[workspace_idx].focused_slide;
        self.desktop.workspaces[workspace_idx]
            .slides
            .get_mut(slide_idx)
            .expect("active slide index out of range")
    }
}
