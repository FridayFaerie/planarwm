use memmap2::MmapMut;
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::{ffi::CString, fs::File, os::fd::AsFd};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::WlCompositor,
        wl_shm::{self, WlShm},
        wl_shm_pool::{self, WlShmPool},
        wl_surface::WlSurface,
    },
};

use crate::{
    AppData,
    river::{
        river_node_v1::RiverNodeV1, river_shell_surface_v1::RiverShellSurfaceV1,
        river_window_manager_v1::RiverWindowManagerV1,
    },
};

#[derive(Debug)]
pub struct Background {
    file: File,

    pub wl_surface: WlSurface,
    pub shell_surface: RiverShellSurfaceV1,
    pub node: RiverNodeV1,

    // TODO: make private
    pub buffer: WlBuffer,

    pub width: u32,
    pub height: u32,
    stride: u32,

    pub shm_data: MmapMut,
}

impl Background {
    pub fn new(
        compositor: &WlCompositor,
        shm: &WlShm,
        river_wm: &RiverWindowManagerV1,
        qh: &QueueHandle<AppData>,
        width: u32,
        height: u32,
    ) -> Self {
        let wl_surface = compositor.create_surface(qh, ());
        let shell_surface = river_wm.get_shell_surface(&wl_surface, qh, ());
        let node = shell_surface.get_node(qh, ());

        let stride = width * 4;
        let size = stride * height;

        let name = CString::new("background").unwrap();

        let fd = memfd_create(name.as_c_str(), MemfdFlags::CLOEXEC).unwrap();

        ftruncate(&fd, size as u64).unwrap();

        let file: File = fd.into();

        let shm_data = unsafe { MmapMut::map_mut(&file).unwrap() };

        let pool: WlShmPool = shm.create_pool(file.as_fd(), size as i32, qh, ());

        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        pool.destroy();

        Background {
            file,
            wl_surface,
            shell_surface,
            node,
            buffer,
            width,
            height,
            stride,
            shm_data,
        }
    }

    pub fn draw_solid(&mut self, color: u32) {
        let pixels = self.stride * self.height / 4;
        let bytes = color.to_ne_bytes();

        for i in 0..pixels {
            let offset = (i * 4) as usize;
            self.shm_data[offset..offset + 4].copy_from_slice(&bytes);
        }
    }

    pub fn draw<F>(&mut self, mut f: F)
    where
        F: FnMut(u32, u32) -> u32,
    {
        for y in 0..self.height {
            for x in 0..self.width {
                let color = f(x, y);
                let offset = (y * self.stride + x * 4) as usize;
                self.shm_data[offset..offset + 4].copy_from_slice(&color.to_ne_bytes());
            }
        }
    }

    pub fn commit(&self) {
        self.wl_surface.attach(Some(&self.buffer), 0, 0);

        self.wl_surface
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        self.wl_surface.commit();
    }

    pub fn sync_commit(&self) {
        self.shell_surface.sync_next_commit();
        self.commit();
    }
}

impl Dispatch<WlShmPool, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        _event: wl_shm_pool::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlBuffer,
        event: wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_buffer::Event::Release => {
                println!("planarwm: compositor released the buffer")
            }
            _ => {}
        }
    }
}
