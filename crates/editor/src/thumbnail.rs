//! The project's picture on the start screen (`thumbnail.png` in its
//! folder), 16:9.
//!
//! By default it's a picture of the scene from the editor's camera, taken
//! at each save and when the project closes: the game's own draw list (no
//! grid, gizmos, light icons or selection outlines), drawn offscreen by a
//! renderer of its own on a thread, so the editor never waits for it. It
//! used to be a screenshot of the window cut to the viewport, which caught
//! the editor's overlays and, waiting on a screenshot event that didn't
//! always come, often wasn't taken at all.
//!
//! Or it's an image the person chose (Project Settings > Choose Image…),
//! cut to 16:9 in the middle; the manifest's `chosen_picture` then stops
//! saves from replacing it.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use axcenity_render::{DrawList, GraphicsSettings, TextureRenderer, wgpu};

/// The picture's size in pixels: twice the start screen's card, so it's
/// sharp on a high-density display.
pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 360;

/// A picture being drawn on its own thread, written to the project's
/// folder when done.
pub struct PictureJob {
    done: Receiver<Result<(), String>>,
}

impl PictureJob {
    /// Draws `list` (built from the editor's camera) with `graphics` and
    /// saves it as `folder`'s picture, on a thread. `None` if the thread
    /// couldn't start.
    pub fn start(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        list: DrawList,
        graphics: GraphicsSettings,
        folder: PathBuf,
    ) -> Option<PictureJob> {
        let (send, done) = mpsc::channel();
        let (device, queue) = (device.clone(), queue.clone());
        std::thread::Builder::new()
            .name("project picture".to_owned())
            .spawn(move || {
                let result =
                    draw(&device, &queue, &list, &graphics).and_then(|png| save(&folder, &png));
                let _ = send.send(result);
            })
            .ok()?;
        Some(PictureJob { done })
    }

    /// The job's outcome once it's finished.
    pub fn poll(&self) -> Option<Result<(), String>> {
        match self.done.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("the picture's thread stopped".into())),
        }
    }
}

/// `list` drawn at the picture's size, as PNG bytes. Drawn until its
/// meshes and textures have reached the GPU, then twice more so temporal
/// anti-aliasing settles.
pub fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    list: &DrawList,
    graphics: &GraphicsSettings,
) -> Result<Vec<u8>, String> {
    let mut renderer = TextureRenderer::new(device, queue, WIDTH, HEIGHT);
    renderer.set_graphics(graphics);
    let mut settled = 0;
    for _ in 0..30 {
        renderer
            .render(list)
            .map_err(|error| format!("couldn't draw the scene: {error:#}"))?;
        if renderer.pending_uploads() == 0 {
            settled += 1;
            if settled == 3 {
                break;
            }
        }
    }
    png(&renderer.read_pixels(), WIDTH, HEIGHT)
}

/// Writes `png` as `folder`'s picture.
pub fn save(folder: &Path, png: &[u8]) -> Result<(), String> {
    std::fs::write(folder.join(axcenity_project::THUMBNAIL_FILE), png)
        .map_err(|error| format!("couldn't save the picture: {error}"))
}

/// An image file's bytes as the project's picture: the largest 16:9 area
/// in its middle, at the picture's size, as PNG bytes.
pub fn fit(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("couldn't read the image: {error}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let (crop_width, crop_height) =
        if u64::from(width) * u64::from(HEIGHT) > u64::from(height) * u64::from(WIDTH) {
            (height * WIDTH / HEIGHT, height)
        } else {
            (width, width * HEIGHT / WIDTH)
        };
    if crop_width == 0 || crop_height == 0 {
        return Err("the image is empty".to_owned());
    }
    let cropped = image::imageops::crop_imm(
        &image,
        (width - crop_width) / 2,
        (height - crop_height) / 2,
        crop_width,
        crop_height,
    )
    .to_image();
    let resized = image::imageops::resize(
        &cropped,
        WIDTH,
        HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );
    png(resized.as_raw(), WIDTH, HEIGHT)
}

/// RGBA rows as PNG bytes (the alpha left out: the picture is opaque).
fn png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&rgb))
        .map_err(|error| format!("couldn't encode the picture: {error}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let image = image::RgbaImage::from_fn(width, height, |x, y| image::Rgba(pixel(x, y)));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    /// The scene is drawn offscreen, at the picture's size, without the
    /// editor's overlays, and saved where the start screen looks for it.
    #[test]
    fn the_scene_is_drawn_and_saved_as_the_projects_picture() {
        let instance = axcenity_render::adapters::instance();
        let Ok(adapter) = pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
        else {
            eprintln!("no GPU: skipped");
            return;
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let mut world = axcenity_core::World::new();
        let workspace = world.workspace();
        let part = world.create(axcenity_core::ClassName::Part).unwrap();
        world.set_parent(part, Some(workspace)).unwrap();
        world
            .set_property(
                part,
                "Size",
                axcenity_core::Value::Vector3(axcenity_core::Vector3::new(8.0, 8.0, 8.0)),
            )
            .unwrap();
        let camera = axcenity_render::Camera {
            position: axcenity_render::glam::Vec3::new(14.0, 10.0, 14.0),
            look_at: axcenity_render::glam::Vec3::ZERO,
            fov_y_degrees: 70.0,
        };
        let list =
            axcenity_scene::build_draw_list(&world, &axcenity_assets::Assets::default(), camera);
        let folder = std::env::temp_dir().join(format!("axcenity-picture-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let job = PictureJob::start(
            &device,
            &queue,
            list,
            GraphicsSettings::default(),
            folder.clone(),
        )
        .unwrap();
        let started = std::time::Instant::now();
        let result = loop {
            if let Some(result) = job.poll() {
                break result;
            }
            assert!(started.elapsed().as_secs() < 120, "the picture never came");
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        result.unwrap();
        let bytes = std::fs::read(folder.join(axcenity_project::THUMBNAIL_FILE)).unwrap();
        let picture = eframe::icon_data::from_png_bytes(&bytes).unwrap();
        assert_eq!((picture.width, picture.height), (WIDTH, HEIGHT));
        // The part in the middle isn't the sky at the top.
        let at = |x: u32, y: u32| {
            let i = ((y * WIDTH + x) * 4) as usize;
            picture.rgba[i..i + 3].to_vec()
        };
        assert_ne!(at(WIDTH / 2, HEIGHT / 2), at(WIDTH / 2, 2));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_chosen_image_is_cut_to_16_by_9_in_its_middle() {
        // A square, red in its top and bottom eighths: cut to 16:9 in the
        // middle, the red is gone.
        let square = encoded(800, 800, |_, y| {
            if !(100..700).contains(&y) {
                [255, 0, 0, 255]
            } else {
                [10, 20, 30, 255]
            }
        });
        let picture = fit(&square).unwrap();
        let decoded = eframe::icon_data::from_png_bytes(&picture).unwrap();
        assert_eq!((decoded.width, decoded.height), (WIDTH, HEIGHT));
        for row in [0, HEIGHT - 1] {
            let at = (row * WIDTH * 4) as usize;
            assert_eq!(&decoded.rgba[at..at + 4], &[10, 20, 30, 255], "row {row}");
        }
        // A wide one keeps its full height.
        let wide = encoded(1000, 200, |_, _| [1, 2, 3, 255]);
        let decoded = eframe::icon_data::from_png_bytes(&fit(&wide).unwrap()).unwrap();
        assert_eq!((decoded.width, decoded.height), (WIDTH, HEIGHT));
        assert!(fit(b"not an image").is_err());
    }
}
