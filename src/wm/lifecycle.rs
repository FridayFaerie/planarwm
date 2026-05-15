use super::{SeatOp, WindowManager};
use crate::AppData;
use crate::actions::{Action, parse_action, parse_keysym, parse_modifiers};
use crate::config::{Config, WindowConfig};
use crate::protocol::river::wayland_client::Proxy;
use crate::river::{
    river_seat_v1::Modifiers, river_window_manager_v1::RiverWindowManagerV1,
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use crate::wm::RiverWindowV1;
use crate::wm::desktop::Desktop;
use crate::wm::slide::SlideType;
use crate::wm::task::{Phase, Task};
use crate::wm::utils::Position;
use std::collections::HashMap;
use std::sync::mpsc;
use wayland_client::QueueHandle;

impl WindowManager {
    pub fn new() -> WindowManager {
        let (queue_tx, queue_rx) = mpsc::channel();
        WindowManager {
            desktop: Desktop::new(queue_tx.clone()),
            windows: HashMap::new(),
            outputs: HashMap::new(),
            seats: HashMap::new(),
            libinput_devices: HashMap::new(),
            queue_tx,
            queue_rx,
            camera_pos: Position { x: 0, y: 0 },
        }
    }
    pub fn tick_tasks(&mut self, phase: Phase) -> usize {
        let mut pending = Vec::new();

        while let Ok(task) = self.queue_rx.try_recv() {
            pending.push(task);
        }

        let queue_length = pending.len();

        for mut task in pending {
            // TODO: is cloning here the right move?
            if !task.step(self, phase, self.queue_tx.clone()) {
                self.queue_tx.send(task).expect("Couldn't requeue task!");
            }
        }

        queue_length
    }

    pub fn handle_manage_start(
        &mut self,
        proxy: &RiverWindowManagerV1,
        river_xkb: &RiverXkbBindingsV1,
        qh: &QueueHandle<AppData>,
        config: &Config,
    ) {
        if self.tick_tasks(Phase::Manage) > 0 {
            proxy.manage_dirty();
        }

        self.remove_outputs();
        self.remove_seats();
        self.remove_windows();

        self.init_new_seats(river_xkb, qh, config);
        self.manage_seats(proxy);

        self.init_new_windows(&config.window);
        // TODO: remove
        // self.manage_layout();
        self.manage_windows();

        // TODO: move this block into its own function?
        for workspace in self.desktop.workspaces.values_mut() {
            if workspace.focus_active_requested {
                // set camera focus to active slide
                // active_slide.focus_nearest()
                // if there are windows in active slide, seat.focus_window
                // rearrange workspace's children
                let active_slide = &mut workspace.slides[workspace.active_slide];
                self.camera_pos = active_slide.position;
                // TODO: next_window comes here, can I refactor focus_nearest somewhere else?
                // active_slide.focus_nearest();

                // TODO: add config option to remove this (keyboard focus on slide change)
                // TODO: idt I should do this weird check
                if !active_slide.windows.is_empty() {
                    for seat in self.seats.values_mut() {
                        seat.focus_window(&active_slide.windows[active_slide.active_window])
                    }
                }

                workspace.focus_active_requested = false;

                workspace.child_rearrange_required = true;
            }
        }
        self.set_window_node_positions();
        proxy.manage_finish();
    }

    pub fn handle_render_start(&mut self, proxy: &RiverWindowManagerV1) {
        self.tick_tasks(Phase::Render);

        for seat in self.seats.values_mut() {
            match &seat.op {
                SeatOp::None => {}
                SeatOp::Pan { start_camera_pos} => {
                    // TODO: why isn't this auto-formatting?
                    self.camera_pos =  *start_camera_pos - seat.op_diff * 2.0;
                }
                SeatOp::Move {
                    // window_proxy,
                    // start_x,
                    // start_y,
                    ..
                } => {
                    // if let Some(window) = self
                    //     .windows
                    //     .values_mut()
                    //     .find(|window| &window.proxy == window_proxy)
                    // {
                    //     let x = start_x + seat.op_dx;
                    //     let y = start_y + seat.op_dy;
                    //     window.set_position(x, y);
                    // }
                }
                // This code "saves" the position as the resize goes on
                SeatOp::Resize {
                    window_proxy,
                    // start_x,
                    // start_y,
                    // start_width,
                    // start_height,
                    // edges,
                    ..
                } => {
                    if let Some(_window) = self
                        .windows
                        .values_mut()
                        .find(|window| &window.proxy == window_proxy)
                    {
                        // let (mut x, mut y) = (*start_x, *start_y);
                        // if edges.contains(Edges::Left) {
                        //     x += start_width - window.width;
                        // }
                        // if edges.contains(Edges::Top) {
                        //     y += start_height - window.height;
                        // }
                        // window.set_position(x, y);
                    }
                }
            }
        }

        self.set_window_node_positions();

        // TODO: this is kinda overdoing it with the set_window_node_positions code
        for seat in self.seats.values_mut() {
            if seat.op != SeatOp::None {
                for window in self.windows.values_mut() {
                    window.set_node_position(self.camera_pos);
                }
            }
        }

        proxy.render_finish();
    }

    pub fn set_window_node_positions(&mut self) {
        // TODO: is there a way to not do this so frequently?
        // TODO: is there a better way to do this?
        for window in self.windows.values_mut() {
            if let Some(render_position) = window.render_position.take() {
                window.node.set_position(
                    render_position.x - self.camera_pos.x,
                    render_position.y - self.camera_pos.y,
                );
            }
        }
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
        // TODO: replace this with a task
        self.windows = old_windows
            .into_iter()
            .filter(|(_, window)| {
                if window.closed {
                    for seat in self.seats.values_mut() {
                        if let SeatOp::Move { window_proxy, .. }
                        | SeatOp::Resize { window_proxy, .. } = &seat.op
                            && window_proxy == &window.proxy
                        {
                            seat.op_end();
                        }
                    }

                    if let Some(loc) = &window.location
                        && let Some(workspace) = self.desktop.workspaces.get_mut(&loc.workspace_id)
                        && let Some(slide) =
                            workspace.slides.iter_mut().find(|s| s.id == loc.slide_id)
                    {
                        slide.windows.retain(|w| w != &window.proxy);
                        slide.rearrange();
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

    // TODO: new windows init with slide dimensions, not window dimensions
    pub fn init_new_windows(&mut self, window_config: &WindowConfig) {
        // TODO: this seems weird, is there a better way
        let new_window_ids: Vec<RiverWindowV1> = self
            .windows
            .iter()
            .filter(|(_, w)| w.new)
            .map(|(id, _)| id.clone())
            .collect();
        for window_id in new_window_ids {
            self.desktop
                .attach_window(window_id.clone(), &mut self.windows);
            if let Some(window) = self.windows.get_mut(&window_id) {
                // window.proxy.propose_dimensions(window.width, window.height);
                if window_config.force_ssd {
                    window.proxy.use_ssd();
                }
                window.proxy.inform_maximized();
                window.new = false;
                window.node.place_top();
                // window.proxy.set_borders(Edges::all(), 3, 4294967295, 0, 0, 4294967295);
                for seat in self.seats.values_mut() {
                    seat.focus_window(&window_id)
                }
            }
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
            if cfg!(debug_assertions) {
                let mods = Modifiers::Mod4;
                seat.create_pointer_binding(qh, mods, BTN_LEFT, Action::Move);
                seat.create_pointer_binding(qh, mods, BTN_RIGHT, Action::Resize);
                seat.create_pointer_binding(qh, mods, BTN_MIDDLE, Action::Pan);
            } else {
                let mods = Modifiers::Mod1;
                seat.create_pointer_binding(qh, mods, BTN_LEFT, Action::Move);
                seat.create_pointer_binding(qh, mods, BTN_RIGHT, Action::Resize);
                seat.create_pointer_binding(qh, Modifiers::None, BTN_MIDDLE, Action::Pan);
            }

            seat.new = false;
        }
    }

    pub fn manage_windows(&mut self) {
        for window in self.windows.values_mut() {
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
        }
    }

    pub fn manage_seats(&mut self, wm_proxy: &RiverWindowManagerV1) {
        let desktop = &mut self.desktop;
        let windows = &mut self.windows;
        let camera_pos = &mut self.camera_pos;

        for seat in self.seats.values_mut() {
            if let Some(window_proxy) = seat.interacted.take() {
                // TODO: this is unsafe!
                let window = windows.get_mut(&window_proxy).unwrap();
                window.node.place_top();
                seat.focus_window(&window_proxy);

                // TODO: should probably fix this code, this just seems goofy
                if let Some(location) = &window.location {
                    desktop.active_workspace = location.workspace_id.clone();
                    let workspace = desktop.active_workspace_mut();
                    workspace.active_slide = workspace
                        .slides
                        .iter()
                        .position(|s| s.id == location.slide_id)
                        .expect("oops can't find slide");

                    if let Some(slide) = workspace.slides.get_mut(workspace.active_slide) {
                        if slide.slide_type != SlideType::Floating {
                            self.queue_tx
                                .send(Task::MoveCamera {
                                    position: Position {
                                        x: slide.position.x,
                                        y: slide.position.y,
                                    },
                                })
                                .expect("can't send movecamera");
                        }
                        slide.active_window = slide
                            .windows
                            .iter()
                            .position(|w| w == &window_proxy)
                            .expect("can't find active window");
                        slide.rearrange();
                    }
                }
            }

            seat.do_action(desktop, windows, &self.outputs, wm_proxy, camera_pos);
            if seat.op_release {
                seat.op_end();
                seat.op_release = false;
            } else {
                seat.op_manage();
            }
        }
    }
}
