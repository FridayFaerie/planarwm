use bytemuck::cast_slice_mut;
use memmap2::MmapMut;
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::{ffi::CString, fs::File, os::fd::AsFd, path::Path};
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
    wm::utils::Position,
};

#[derive(Debug)]
pub struct Background {
    pub wl_surface: WlSurface,
    pub shell_surface: RiverShellSurfaceV1,
    pub node: RiverNodeV1,

    buffer: WlBuffer,

    width: u32,
    height: u32,

    pub shm_data: MmapMut,

    wallpaper: Wallpaper,
    offset_x: i32,
    offset_y: i32,
}

#[derive(Debug)]
pub struct Wallpaper {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl Background {
    pub fn new(
        compositor: &WlCompositor,
        shm: &WlShm,
        river_wm: &RiverWindowManagerV1,
        qh: &QueueHandle<AppData>,
        width: u32,
        height: u32,
        wallpaper_path: impl AsRef<Path>,
    ) -> Self {
        let wl_surface = compositor.create_surface(qh, ());
        let shell_surface = river_wm.get_shell_surface(&wl_surface, qh, ());
        let node = shell_surface.get_node(qh, ());

        let size = width * height * 4;

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
            (width * 4) as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        pool.destroy();

        let wallpaper = Wallpaper::load(wallpaper_path);

        Background {
            wl_surface,
            shell_surface,
            node,
            buffer,
            width,
            height,
            shm_data,
            wallpaper,

            offset_x: 0,
            offset_y: 0,
        }
    }

    pub fn draw_solid(&mut self, color: u32) {
        let pixels = self.width * self.height;
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
                let offset = (y * self.width * 4 + x * 4) as usize;
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

    // TODO: this is probably stupidly slow, fix someday
    pub fn render(&mut self, camera_pos: Position) {
        let pixels: &mut [u32] = bytemuck::cast_slice_mut(&mut self.shm_data);

        let wallpaper = &self.wallpaper;

        let w = wallpaper.width as i32;
        let h = wallpaper.height as i32;

        let mut sy = camera_pos.y.rem_euclid(h);

        let mut dst_index = 0;

        for _y in 0..self.height {
            let row_start = sy as usize * wallpaper.width as usize;

            let mut sx = camera_pos.x.rem_euclid(w);

            for _x in 0..self.width {
                pixels[dst_index] = wallpaper.pixels[row_start + sx as usize];

                dst_index += 1;

                sx += 1;

                if sx >= w {
                    sx = 0;
                }
            }

            sy += 1;

            if sy >= h {
                sy = 0;
            }
        }
    }
}

impl Wallpaper {
    pub fn load(path: impl AsRef<Path>) -> Self {
        let image = image::open(path)
            .expect("failed to open wallpaper")
            .to_rgba8();

        let (width, height) = image.dimensions();

        let mut pixels = Vec::with_capacity((width * height) as usize);

        for pixel in image.pixels() {
            let [r, g, b, a] = pixel.0;
            let argb = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            pixels.push(argb);
        }

        Self {
            width,
            height,
            pixels,
        }
    }

    #[inline]
    pub fn sample(&self, x: i32, y: i32) -> u32 {
        let tx = x.rem_euclid(self.width as i32) as u32;
        let ty = y.rem_euclid(self.height as i32) as u32;
        self.pixels[(ty * self.width + tx) as usize]
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
                // println!("planarwm: compositor released the buffer")
            }
            _ => {}
        }
    }
}
