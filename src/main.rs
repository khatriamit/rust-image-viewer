use std::cmp::max;

use clap::Parser;
use image::ImageError;
use pixels::{Pixels, SurfaceTexture};
use thiserror::Error;
use winit::{
    dpi::PhysicalSize,
    error::OsError,
    event::{ElementState, KeyboardInput, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
#[derive(Debug, Error)]
enum RvuError {
    #[error("unable to create window.")]
    WindowError(#[from] OsError),

    #[error("error occured while processing the image")]
    ImageError(#[from] ImageError),

    #[error("An error while loading image")]
    IoError(#[from] std::io::Error),

    #[error("unable to calculate the maximum size on your monitor")]
    NoPrimaryMonitor,

    #[error("unable to create pixel buffer to display image")]
    PixelError(#[from] pixels::Error),
}
#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Config {
    /// Name of the image to view
    file_name: String,
}

type Result<T> = std::result::Result<T, RvuError>;

const SCREEN_PERCENT: u32 = 90;

fn main() -> Result<()> {
    let config = Config::parse();
    let image = image::io::Reader::open(&config.file_name)?.decode()?;
    println!("Filename {}", config.file_name);

    let event_loop = EventLoop::new();
    let primary_monitor = event_loop
        .primary_monitor()
        .ok_or(RvuError::NoPrimaryMonitor)?;
    let screen_size = primary_monitor.size();
    let max_screen_size = (
        screen_size.width * SCREEN_PERCENT / 100,
        screen_size.height * SCREEN_PERCENT / 100,
    );

    let horz_scale = calc_scale(max_screen_size.0, image.width());
    let vert_scale = calc_scale(max_screen_size.1, image.height());
    let scale = max(horz_scale, vert_scale);

    let window_inner_size = PhysicalSize::new(image.width() / scale, image.height() / scale);

    let window = WindowBuilder::new()
        .with_title("RVU")
        .with_inner_size(window_inner_size)
        .build(&event_loop)?;

    let surface = SurfaceTexture::new(window_inner_size.width, window_inner_size.height, &window);
    let mut pixels = Pixels::new(image.width(), image.height(), surface)?;
    let image_bytes = image.as_rgb8().unwrap().as_flat_samples();
    let image_bytes = image_bytes.as_slice();
    let pixel_bytes = pixels.get_frame_mut();

    image_bytes
        .chunks_exact(3)
        .zip(pixel_bytes.chunks_exact_mut(4))
        .for_each(|(image_pixel, pixels)| {
            pixels[0] = image_pixel[0];
            pixels[1] = image_pixel[1];
            pixels[2] = image_pixel[2];
            pixels[3] = 0xff;
        });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            winit::event::Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match event {
                    winit::event::WindowEvent::Resized(size) => resize(&mut pixels, &size),
                    winit::event::WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    winit::event::WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        resize(&mut pixels, new_inner_size);
                    }
                    _ => {}
                }
            }
            winit::event::Event::RedrawRequested(_) => {
                let _ = pixels.render();
            }
            _ => {}
        }
    });
}

fn calc_scale(max_size: u32, current_size: u32) -> u32 {
    if max_size >= current_size {
        1
    } else {
        ((current_size as f32) / (max_size as f32)).ceil() as u32
    }
}

fn resize(pixels: &mut Pixels, size: &PhysicalSize<u32>) {
    pixels.resize_surface(size.width, size.height);
}
