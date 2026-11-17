//! World streaming's background uploads (Phase 80): meshes and textures
//! come in on their own thread, and what stops being drawn leaves the GPU.

use super::*;
use glam::Mat4;

struct Tile {
    key: u64,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl MeshSource for Tile {
    fn key(&self) -> u64 {
        self.key
    }
    fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }
    fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[]
    }
    fn indices(&self) -> &[u32] {
        &self.indices
    }
}

/// A 4 x 4 image.
struct Picture(u64, Vec<u8>);

impl TextureSource for Picture {
    fn key(&self) -> u64 {
        self.0
    }
    fn size(&self) -> (u32, u32) {
        (4, 4)
    }
    fn rgba(&self) -> &[u8] {
        &self.1
    }
}

fn mesh_item(key: u64) -> DrawItem {
    let tile = Tile {
        key,
        positions: vec![
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    };
    let mut item = big_block();
    item.shape = Shape::Mesh(Arc::new(tile));
    item.transform = Mat4::from_translation(Vec3::new(0.0, 0.0, -6.0));
    item
}

/// Drawing `list` every frame until nothing is on its way.
fn until_uploaded(renderer: &mut TextureRenderer, list: &DrawList) {
    for _ in 0..500 {
        renderer.render(list).unwrap();
        if renderer.pending_uploads() == 0 {
            renderer.render(list).unwrap();
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    panic!("the uploads never finished");
}

#[test]
fn a_mesh_is_uploaded_in_the_background_and_drawn_a_few_frames_later() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 64, 64);
    renderer.set_background_uploads(true);
    let list = list_of(vec![mesh_item(901)]);
    renderer.render(&list).unwrap();
    assert!(
        renderer.scene.gpu_mesh(MeshKey::Custom(901)).is_none(),
        "the frame that first draws it doesn't wait for it"
    );
    assert_eq!(renderer.pending_uploads(), 1);
    until_uploaded(&mut renderer, &list);
    assert!(renderer.scene.gpu_mesh(MeshKey::Custom(901)).is_some());
}

#[test]
fn what_stops_being_drawn_leaves_the_gpu_and_uploads_again_on_the_way_back() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 64, 64);
    renderer.set_background_uploads(true);
    let shown = list_of(vec![mesh_item(902)]);
    until_uploaded(&mut renderer, &shown);
    let bytes = renderer.scene.mesh_bytes.total();
    assert!(bytes > 0);

    // Many frames without it being drawn, but quick ones: within
    // `KEEP_UNDRAWN_SECONDS` it stays, so turning back doesn't upload it
    // again.
    let empty = list_of(Vec::new());
    renderer.scene.frame += KEEP_UNDRAWN_FRAMES;
    renderer.render(&empty).unwrap();
    assert!(
        renderer.scene.gpu_mesh(MeshKey::Custom(902)).is_some(),
        "kept while it was drawn only seconds ago"
    );

    // A long time passes without it being drawn: the frames and the
    // seconds (the clock of recent frames forgets them).
    renderer.scene.frame += KEEP_UNDRAWN_FRAMES;
    renderer.scene.frame_clock.clear();
    renderer.render(&empty).unwrap();
    assert!(
        renderer.scene.gpu_mesh(MeshKey::Custom(902)).is_none(),
        "unloaded once it's been out of range a while"
    );

    until_uploaded(&mut renderer, &shown);
    assert!(
        renderer.scene.gpu_mesh(MeshKey::Custom(902)).is_some(),
        "loaded again when it comes back"
    );
}

#[test]
fn a_texture_comes_in_on_the_thread_and_replaces_the_white_it_drew_with() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 64, 64);
    renderer.set_background_uploads(true);
    let pixels = [200u8, 30, 30, 255].repeat(16);
    let item = textured_block(Arc::new(Picture(903, pixels)));
    let list = list_of(vec![item]);
    renderer.render(&list).unwrap();
    assert!(
        !renderer.scene.textures.contains_key(&903),
        "not on the GPU yet: it draws white"
    );
    until_uploaded(&mut renderer, &list);
    assert!(renderer.scene.textures.contains_key(&903));
}

/// Flat ground 20 studs across, as a terrain chunk is at one level of
/// detail: each level is a mesh of its own key.
struct ChunkLevel(u64);

impl MeshSource for ChunkLevel {
    fn key(&self) -> u64 {
        self.0
    }
    fn positions(&self) -> &[[f32; 3]] {
        &[
            [-10.0, 0.0, -10.0],
            [-10.0, 0.0, 10.0],
            [10.0, 0.0, -10.0],
            [10.0, 0.0, 10.0],
        ]
    }
    fn normals(&self) -> &[[f32; 3]] {
        &[[0.0, 1.0, 0.0]; 4]
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
    }
    fn indices(&self) -> &[u32] {
        &[0, 1, 2, 1, 3, 2]
    }
}

#[test]
fn a_terrain_chunk_switching_level_is_drawn_in_the_frame_it_switches() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // As in the editor, which always uploads in the background.
    let mut renderer = TextureRenderer::new(&device, &queue, 64, 64);
    renderer.set_background_uploads(true);
    let layers = Arc::new(crate::TerrainLayers {
        id: 1,
        key: 1,
        layers: vec![crate::TerrainLayer {
            color: [1.0, 0.0, 0.0],
            ..terrain::tests::layer()
        }],
        view: crate::TerrainView::Shaded,
        height_range: [0.0, 1.0],
        grid_origin: [-10.0, -10.0],
        grid_cell: [1.0, 1.0],
    });
    let weights: [Arc<dyn TextureSource>; 2] = [
        Arc::new(Picture(911, [255, 0, 0, 0].repeat(16))),
        Arc::new(Picture(912, vec![0; 64])),
    ];
    let chunk_at_level = |key| {
        let mut item = big_block();
        item.shape = Shape::Mesh(Arc::new(ChunkLevel(key)));
        item.transform = Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0));
        item.material = Material {
            terrain: Some(crate::TerrainSurface {
                layers: layers.clone(),
                weights: weights.clone(),
            }),
            ..Material::default()
        };
        let mut list = list_of(vec![item]);
        // Looking straight down at it, in sunlight.
        list.camera.look_at = Vec3::new(0.0, -1.0, -0.001);
        list.ambient.intensity = 0.2;
        list
    };
    let middle = |renderer: &TextureRenderer| {
        let pixels = renderer.read_pixels();
        let at = (32 * 64 + 32) * 4;
        [pixels[at], pixels[at + 1], pixels[at + 2]]
    };
    // The camera moves and the chunk's level changes every frame: each
    // level is drawn in the frame that first asks for it, never a frame
    // with a hole where the ground was.
    for key in [921, 922, 923, 922, 924] {
        renderer.render(&chunk_at_level(key)).unwrap();
        assert!(
            renderer.scene.gpu_mesh(MeshKey::Custom(key)).is_some(),
            "level {key} is on the GPU in the frame that draws it"
        );
        let [red, green, blue] = middle(&renderer);
        assert!(
            red > 100 && red > blue * 3 && red > green * 3,
            "level {key} shows the ground: {red}, {green}, {blue}"
        );
    }
    assert_eq!(renderer.pending_uploads(), 0, "nothing waits on the thread");
}
