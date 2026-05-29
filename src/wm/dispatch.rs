use super::{LayerFocus, Output, Seat, Window};
use crate::AppData;
pub use crate::protocol::river;
use crate::river::river_shell_surface_v1::RiverShellSurfaceV1;
use crate::river::{
    river_input_device_v1::RiverInputDeviceV1, river_input_manager_v1::RiverInputManagerV1,
    river_layer_shell_output_v1::RiverLayerShellOutputV1,
    river_layer_shell_seat_v1::RiverLayerShellSeatV1, river_layer_shell_v1::RiverLayerShellV1,
    river_libinput_config_v1::RiverLibinputConfigV1,
    river_libinput_device_v1::RiverLibinputDeviceV1,
    river_libinput_result_v1::RiverLibinputResultV1, river_node_v1::RiverNodeV1,
    river_output_v1::RiverOutputV1, river_pointer_binding_v1::RiverPointerBindingV1,
    river_seat_v1::RiverSeatV1, river_window_manager_v1::RiverWindowManagerV1,
    river_window_v1::RiverWindowV1, river_xkb_binding_v1::RiverXkbBindingV1,
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use crate::wm::LibinputDevice;
use crate::wm::background::Background;
use crate::wm::task::Task;
use crate::wm::utils::Position;
use wayland_backend::client::ObjectId;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, protocol::wl_registry};

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            const RIVER_WINDOW_MANAGER_V1_VERSION: u32 = 4;
            const RIVER_XKB_BINDINGS_V1_VERSION: u32 = 1;
            const RIVER_LAYER_SHELL_V1_VERSION: u32 = 1;
            match interface.as_str() {
                "river_window_manager_v1" => {
                    if version < RIVER_WINDOW_MANAGER_V1_VERSION {
                        eprintln!(
                            "Server river_window_manager_v1 v{version}, but we need at least v{RIVER_WINDOW_MANAGER_V1_VERSION}",
                        );
                        std::process::exit(1);
                    }
                    let wm = registry.bind::<RiverWindowManagerV1, _, _>(
                        name,
                        RIVER_WINDOW_MANAGER_V1_VERSION,
                        qh,
                        (),
                    );
                    state.river_wm = Some(wm);
                }
                "river_xkb_bindings_v1" => {
                    if version < RIVER_XKB_BINDINGS_V1_VERSION {
                        eprintln!(
                            "Server supports river_xkb_bindings_v1 v{version}, but we need at least v{RIVER_XKB_BINDINGS_V1_VERSION}"
                        );
                        std::process::exit(1);
                    }
                    let xkb = registry.bind::<RiverXkbBindingsV1, _, _>(
                        name,
                        RIVER_XKB_BINDINGS_V1_VERSION,
                        qh,
                        (),
                    );
                    state.river_xkb = Some(xkb);
                }
                "river_layer_shell_v1" => {
                    if version < RIVER_LAYER_SHELL_V1_VERSION {
                        eprintln!(
                            "Server supports river_layer_shell_v1 v{version}, but we need at least v{RIVER_LAYER_SHELL_V1_VERSION}"
                        );
                        std::process::exit(1);
                    }
                    let layer_shell = registry.bind::<RiverLayerShellV1, _, _>(
                        name,
                        RIVER_LAYER_SHELL_V1_VERSION,
                        qh,
                        (),
                    );
                    state.river_ls = Some(layer_shell);
                }
                "river_input_manager_v1" => {
                    state.river_im =
                        Some(registry.bind::<RiverInputManagerV1, _, _>(name, 1, qh, ()));
                }
                "river_libinput_config_v1" => {
                    state.river_lc =
                        Some(registry.bind::<RiverLibinputConfigV1, _, _>(name, 1, qh, ()));
                }
                "wl_compositor" => {
                    state.compositor = Some(registry.bind::<WlCompositor, _, _>(name, 6, qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind::<WlShm, _, _>(name, 2, qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlShm, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        _event: <WlShm as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlCompositor, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<RiverWindowManagerV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverWindowManagerV1,
        event: <RiverWindowManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use river::river_window_manager_v1::Event;
        match event {
            Event::Unavailable => {
                eprintln!("Error: Another WM is already running");
                std::process::exit(1);
            }
            Event::Finished => std::process::exit(0),
            Event::ManageStart => {
                let river_xkb = state
                    .river_xkb
                    .as_ref()
                    .expect("river_xkb_bindings_v1 missing");
                // TODO: can probably remove this config cloning someday
                let config = state.config.clone();
                state.wm.handle_manage_start(proxy, river_xkb, qh, &config)
            }
            Event::RenderStart => state.wm.handle_render_start(proxy),
            Event::SessionLocked => {}
            Event::SessionUnlocked => {}
            Event::Window { id } => {
                state.wm.windows.insert(id.id(), Window::new(id, qh));
            }
            Event::Output { id } => {
                let mut output = Output::new(id.clone());
                if let Some(layer_shell) = &state.river_ls {
                    output.layer =
                        Some(layer_shell.get_output(&output.proxy, qh, output.proxy.id()));
                }
                if let Some(compositor) = state.compositor.as_mut()
                    && let Some(river_wm) = state.river_wm.as_mut()
                    && let Some(shm) = state.shm.as_mut()
                    && !state.config.window.wallpaper_path.is_empty()
                {
                    output.background = Some(Background::new(
                        compositor,
                        shm,
                        river_wm,
                        qh,
                        // TODO: change this to like 1,1 or something, and wait for the output's
                        // dimensions event
                        output.dimensions.unwrap_or((1280, 720)).0 as u32,
                        output.dimensions.unwrap_or((1280, 720)).1 as u32,
                        // TODO: add some nicer checking?
                        state.config.window.wallpaper_path.clone(),
                    ));
                }
                state.wm.outputs.insert(id.id(), output);
                state
                    .wm
                    .queue_tx
                    .send(Task::InitNewOutput { id: id.id() })
                    .expect("couldn't send initnewoutput");
                state
                    .wm
                    .queue_tx
                    .send(Task::InitNewBackground { id: id.id() })
                    .expect("couldn't send initnewbackground");
            }
            Event::Seat { id } => {
                let mut seat = Seat::new(id.clone(), state.wm.queue_tx.clone());
                if let Some(layer_shell) = &state.river_ls {
                    seat.layer = Some(layer_shell.get_seat(&seat.proxy, qh, seat.proxy.id()))
                }
                state.wm.seats.insert(id.id(), seat);
            }
        }
    }

    wayland_client::event_created_child!(AppData, RiverWindowManagerV1, [
        river::river_window_manager_v1::EVT_WINDOW_OPCODE => (RiverWindowV1, ()),
        river::river_window_manager_v1::EVT_OUTPUT_OPCODE => (RiverOutputV1, ()),
        river::river_window_manager_v1::EVT_SEAT_OPCODE => (RiverSeatV1, ())
    ]);
}

impl Dispatch<RiverWindowV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverWindowV1,
        event: <RiverWindowV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_window_v1::Event;
        let window = match state
            .wm
            .windows
            .values_mut()
            .find(|output| &output.proxy == proxy)
        {
            Some(window) => window,
            None => return,
        };
        match event {
            Event::Closed => state
                .wm
                .queue_tx
                .send(Task::CloseWindow {
                    window_id: window.proxy.id(),
                })
                .expect("couldn't send closewindow"),
            Event::DimensionsHint {
                min_width: _,
                min_height: _,
                max_width: _,
                max_height: _,
            } => {}
            Event::Dimensions { width, height } => {
                if window.width == width && window.height == height {
                    return;
                }
                window.width = width;
                window.height = height;
                // TODO: remove this if not needed?
                let location = window.location.as_ref().unwrap();
                state
                    .wm
                    .desktop
                    .workspaces
                    .get_mut(&location.workspace_id)
                    .unwrap()
                    .slides
                    .iter_mut()
                    .find(|s| s.id == location.slide_id)
                    .unwrap()
                    .rearrange();
            }
            Event::AppId { app_id } => {
                if let Some(id) = app_id {
                    window.app_id = id;
                }
            }
            Event::Title { title } => {
                if let Some(window_title) = title {
                    window.title = window_title;
                }
            }
            Event::Parent { parent: _ } => {}
            Event::DecorationHint { hint: _ } => {}
            Event::PointerMoveRequested { seat } => window.pointer_move_requested = Some(seat),
            Event::PointerResizeRequested { seat, edges } => {
                window.pointer_resize_requested = Some(seat);
                window.pointer_resize_requested_edges = edges
                    .into_result()
                    .expect("Invalid edges for resize: {edges}");
            }
            Event::ShowWindowMenuRequested { x: _, y: _ } => {}
            Event::MaximizeRequested => {}
            // state
            //     .wm
            //     .queue_tx
            //     .send(Task::MaximizeWindow {
            //         window_id: window.proxy.clone(),
            //     })
            //     .expect("couldn't send maximizewindow"),
            Event::UnmaximizeRequested => {}
            // state
            //     .wm
            //     .queue_tx
            //     .send(Task::MaximizeWindow {
            //         window_id: window.proxy.clone(),
            //     })
            //     .expect("couldn't send maximizewindow"),
            Event::FullscreenRequested { output: _ } => {}
            Event::ExitFullscreenRequested => {}
            Event::MinimizeRequested => {}
            Event::UnreliablePid { unreliable_pid: _ } => {}
            Event::PresentationHint { .. } => {}
            Event::Identifier { .. } => {}
        }
    }
}

impl Dispatch<RiverOutputV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverOutputV1,
        event: <RiverOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use river::river_output_v1::Event;
        let output = state
            .wm
            .outputs
            .get_mut(&proxy.id())
            .expect("Output {proxy.id()} not found");
        match event {
            Event::Removed => output.removed = true,
            Event::WlOutput { name: _ } => {}
            Event::Position { x, y } => output.position = Some((x, y)),
            Event::Dimensions { width, height } => {
                if let Some(background) = &mut output.background {
                    background.node.destroy();
                    background.shell_surface.destroy();
                    background.wl_surface.destroy();
                }
                if let Some(compositor) = state.compositor.as_mut()
                    && let Some(river_wm) = state.river_wm.as_mut()
                    && let Some(shm) = state.shm.as_mut()
                    && !state.config.window.wallpaper_path.is_empty()
                {
                    output.background = Some(Background::new(
                        compositor,
                        shm,
                        river_wm,
                        qh,
                        width as u32,
                        height as u32,
                        // TODO: add some nicer checking?
                        state.config.window.wallpaper_path.clone(),
                    ));

                    state
                        .wm
                        .queue_tx
                        .send(Task::InitNewBackground {
                            id: output.proxy.id(),
                        })
                        .expect("couldn't send initnewbackground");
                }
                if !output.overview_active {
                    println!("changing output dimensions");
                    output.dimensions = Some((width, height));
                    for workspace in state.wm.desktop.workspaces.values_mut() {
                        workspace.dimensions = (width, height);
                        for slide in workspace.slides.iter_mut() {
                            slide.dimensions = (width, height);
                        }
                    }
                } else {
                    println!("not changing output dimensions - overview is active!")
                }
            }
        }
    }
}

impl Dispatch<RiverSeatV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverSeatV1,
        event: <RiverSeatV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_seat_v1::Event;
        let seat = state
            .wm
            .seats
            .get_mut(&proxy.id())
            .expect("Seat {proxy.id()} not found");
        match event {
            Event::Removed => seat.removed = true,
            Event::WlSeat { name: _ } => {}
            Event::PointerEnter { window } => seat.hovered = Some(window.id()),
            Event::PointerLeave => seat.hovered = None,
            Event::WindowInteraction { window } => seat.interacted = Some(window.id()),
            Event::ShellSurfaceInteraction {
                shell_surface: _shell_surface,
            } => {}
            Event::OpDelta { dx, dy } => seat.op_diff = Position { x: dx, y: dy },
            Event::OpRelease => seat.op_release = true,
            Event::PointerPosition { x: _, y: _ } => {}
        }
    }
}

impl Dispatch<RiverLayerShellV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &RiverLayerShellV1,
        _event: <RiverLayerShellV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<RiverLayerShellOutputV1, ObjectId> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &RiverLayerShellOutputV1,
        event: <RiverLayerShellOutputV1 as Proxy>::Event,
        _data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_layer_shell_output_v1::Event;
        match event {
            Event::NonExclusiveArea {
                x: _,
                y: _,
                width: _,
                height: _,
            } => {}
        }
    }
}

impl Dispatch<RiverLayerShellSeatV1, ObjectId> for AppData {
    fn event(
        state: &mut Self,
        _proxy: &RiverLayerShellSeatV1,
        event: <RiverLayerShellSeatV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_layer_shell_seat_v1::Event;
        let seat = state
            .wm
            .seats
            .get_mut(data)
            .expect("Seat {proxy.id()} not found");
        match event {
            Event::FocusExclusive => {
                seat.layer_focus = LayerFocus::Exclusive;
            }
            Event::FocusNonExclusive => {
                seat.layer_focus = LayerFocus::NonExclusive;
            }
            Event::FocusNone => {
                seat.layer_focus = LayerFocus::None;
            }
        }
    }
}

impl Dispatch<RiverXkbBindingV1, ObjectId> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverXkbBindingV1,
        event: <RiverXkbBindingV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_xkb_binding_v1::Event;
        let seat = state.wm.seats.get_mut(data).expect("Seat {data} not found");
        let binding = seat
            .xkb_bindings
            .get(&proxy.id())
            .expect("xkb_binding {proxy.id()} not found");
        match event {
            Event::Pressed => seat.pending_action = binding.action.clone(),
            Event::Released => {}
            Event::StopRepeat => {}
        }
    }
}

impl Dispatch<RiverPointerBindingV1, ObjectId> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverPointerBindingV1,
        event: <RiverPointerBindingV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_pointer_binding_v1::Event;
        let seat = state.wm.seats.get_mut(data).expect("Seat {data} not found");
        let binding = seat
            .pointer_bindings
            .get(&proxy.id())
            .expect("xkb_binding {proxy.id()} not found");
        match event {
            Event::Pressed => seat.pending_action = binding.action.clone(),
            Event::Released => {}
        }
    }
}

impl Dispatch<RiverInputManagerV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &RiverInputManagerV1,
        _event: <RiverInputManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }

    wayland_client::event_created_child!(AppData, RiverInputManagerV1, [
        river::river_input_manager_v1::EVT_INPUT_DEVICE_OPCODE => (RiverInputDeviceV1, ())
    ]);
}

impl Dispatch<RiverInputDeviceV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &RiverInputDeviceV1,
        _event: <RiverInputDeviceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<RiverLibinputDeviceV1, ()> for AppData {
    fn event(
        state: &mut Self,
        proxy: &RiverLibinputDeviceV1,
        event: <RiverLibinputDeviceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use river::river_libinput_device_v1::Event;

        let dev = state
            .wm
            .libinput_devices
            .entry(proxy.id())
            .or_insert_with(|| LibinputDevice::new(proxy.clone()));

        match event {
            Event::TapSupport { finger_count } => {
                dev.tap_support = Some(finger_count);

                if finger_count > 0 {
                    dev.proxy
                        .set_tap(river::river_libinput_device_v1::TapState::Enabled, qh, ());
                    dev.proxy
                        .set_drag(river::river_libinput_device_v1::DragState::Enabled, qh, ());
                    dev.proxy.set_natural_scroll(
                        river::river_libinput_device_v1::NaturalScrollState::Enabled,
                        qh,
                        (),
                    );
                }
            }
            Event::Removed => {
                state.wm.libinput_devices.remove(&proxy.id());
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(AppData, RiverLibinputConfigV1, [
        river::river_libinput_config_v1::EVT_LIBINPUT_DEVICE_OPCODE => (RiverLibinputDeviceV1, ())
    ]);
}

impl Dispatch<RiverLibinputConfigV1, ()> for AppData {
    fn event(
        state: &mut Self,
        _proxy: &RiverLibinputConfigV1,
        event: <RiverLibinputConfigV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_libinput_config_v1::Event;

        match event {
            Event::LibinputDevice { id } => {
                state
                    .wm
                    .libinput_devices
                    .insert(id.id(), LibinputDevice::new(id));
            }
            Event::Finished => {}
        }
    }
    wayland_client::event_created_child!(AppData,RiverLibinputConfigV1,[river::river_libinput_config_v1::EVT_LIBINPUT_DEVICE_OPCODE=>(RiverLibinputDeviceV1,())]);
}

impl Dispatch<RiverLibinputResultV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &RiverLibinputResultV1,
        event: <RiverLibinputResultV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use river::river_libinput_result_v1::Event;
        match event {
            Event::Success => {}
            Event::Unsupported => eprintln!("libinput setting unsupported on this device"),
            Event::Invalid => {
                eprintln!("libinput setting invalid")
            }
        }
    }
}

impl Dispatch<RiverShellSurfaceV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &RiverShellSurfaceV1,
        _event: <RiverShellSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(AppData: ignore RiverXkbBindingsV1);
wayland_client::delegate_noop!(AppData: ignore RiverNodeV1);
