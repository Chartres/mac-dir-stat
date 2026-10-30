//! Renders the app headless, without a GPU, to PNG files under
//! `docs/screenshots/`. A small CPU rasteriser draws egui's tessellated
//! meshes; the window is the real `App::update` over a made-up home folder.
//!
//!     cargo run --example screenshots            # every shot
//!     cargo run --example screenshots -- main-light
//!
//! PNG encoding goes through ImageMagick's `convert` (PPM in, PNG out) so
//! the crate gains no image dependency.

use egui::epaint::{ClippedPrimitive, Primitive};
use egui::{Color32, ImageData, Pos2, TextureId, ThemePreference};
use mac_dir_stat::app::{App, AppState};
use mac_dir_stat::scanner::tree::{FileTree, NodeId};
use mac_dir_stat::treemap::color::ColorMode;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const W: f32 = 1280.0;
const H: f32 = 800.0;
const PPP: f32 = 2.0;

struct Shot {
    name: &'static str,
    dark: bool,
    setup: fn(&mut AppState),
    /// Pointer position in points, if the shot needs a hover.
    pointer: Option<Pos2>,
}

fn shots() -> Vec<Shot> {
    vec![
        Shot { name: "main-light", dark: false, setup: |_| {}, pointer: None },
        Shot { name: "main-dark", dark: true, setup: |_| {}, pointer: None },
        Shot { name: "inventory", dark: false, setup: inventory, pointer: None },
        Shot { name: "inventory-dark", dark: true, setup: inventory, pointer: None },
        Shot { name: "closing-account", dark: false, setup: closing_account, pointer: None },
    ]
}

fn main() {
    let only: Vec<String> = std::env::args().skip(1).collect();
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/screenshots");
    std::fs::create_dir_all(&out).unwrap();
    for shot in shots() {
        if !only.is_empty() && !only.iter().any(|o| o == shot.name) {
            continue;
        }
        let path = out.join(format!("{}.png", shot.name));
        render(&shot, &path);
        println!("wrote {}", path.display());
    }
}

// ── Fixture ─────────────────────────────────────────────────────────────

const GB: u64 = 1024 * 1024 * 1024;
const MB: u64 = 1024 * 1024;

fn days_ago(d: u64) -> SystemTime {
    SystemTime::now() - Duration::from_secs(d * 86_400)
}

/// Deterministic pseudo-random sizes so every run draws the same map.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

fn files(t: &mut FileTree, dir: NodeId, depth: u16, total: u64, exts: &[&str], n: usize, rng: &mut Lcg) {
    let weights: Vec<u64> = (0..n).map(|_| 1 + rng.next() % 100).collect();
    let sum: u64 = weights.iter().sum();
    for (i, w) in weights.iter().enumerate() {
        let ext = exts[i % exts.len()];
        let name = format!("file-{i:03}.{ext}");
        t.add_file(dir, name.as_bytes(), total * w / sum, Some(ext), days_ago(rng.next() % 400), depth);
    }
}

fn dir(t: &mut FileTree, parent: NodeId, name: &str, depth: u16, age: u64) -> NodeId {
    t.add_dir(parent, name.as_bytes(), false, days_ago(age), depth)
}

pub fn fixture_tree() -> FileTree {
    let mut rng = Lcg(7);
    let mut t = FileTree::new();
    t.rename_root(b"/Users/pavol");
    let r = t.root();
    let lib = dir(&mut t, r, "Library", 1, 0);
    let dev = dir(&mut t, lib, "Developer", 2, 3);
    let xcode = dir(&mut t, dev, "Xcode", 3, 3);
    let dd = dir(&mut t, xcode, "DerivedData", 4, 3);
    files(&mut t, dd, 5, 38 * GB, &["o", "swiftmodule", "dylib"], 24, &mut rng);
    let sim = dir(&mut t, dev, "CoreSimulator", 3, 300);
    let simd = dir(&mut t, sim, "Devices", 4, 300);
    files(&mut t, simd, 5, 6 * GB + 600 * MB, &["img", "db"], 8, &mut rng);
    let caches = dir(&mut t, lib, "Caches", 2, 0);
    files(&mut t, caches, 3, 8 * GB, &["cache", "db", "bin"], 20, &mut rng);
    let docker = dir(&mut t, lib, "Containers", 2, 41);
    let dk = dir(&mut t, docker, "com.docker.docker", 3, 41);
    files(&mut t, dk, 4, 12 * GB + 400 * MB, &["raw"], 2, &mut rng);
    let mail = dir(&mut t, lib, "Mail", 2, 1);
    files(&mut t, mail, 3, 9 * GB + 800 * MB, &["emlx", "mbox"], 30, &mut rng);

    let pics = dir(&mut t, r, "Pictures", 1, 2);
    let photos = dir(&mut t, pics, "Photos Library.photoslibrary", 2, 2);
    files(&mut t, photos, 3, 118 * GB, &["heic", "jpg", "mov"], 60, &mut rng);
    let movies = dir(&mut t, r, "Movies", 1, 30);
    files(&mut t, movies, 2, 64 * GB, &["mov", "mp4"], 18, &mut rng);
    let docs = dir(&mut t, r, "Documents", 1, 4);
    files(&mut t, docs, 2, 41 * GB, &["pdf", "docx", "pages", "xlsx", "key"], 50, &mut rng);
    let dl = dir(&mut t, r, "Downloads", 1, 1);
    files(&mut t, dl, 2, 48 * GB, &["dmg", "zip", "pdf", "pkg"], 30, &mut rng);
    let music = dir(&mut t, r, "Music", 1, 90);
    files(&mut t, music, 2, 22 * GB, &["m4a", "mp3"], 40, &mut rng);

    let code = dir(&mut t, r, "code", 1, 2);
    for (i, p) in ["site", "dashboard", "mobile-app", "notes-api"].iter().enumerate() {
        let proj = dir(&mut t, code, p, 2, 2 + i as u64 * 40);
        let nm = dir(&mut t, proj, "node_modules", 3, 2 + i as u64 * 40);
        files(&mut t, nm, 4, (3 + i as u64 * 2) * GB, &["js", "json", "map"], 30, &mut rng);
        let src = dir(&mut t, proj, "src", 3, 2);
        files(&mut t, src, 4, 120 * MB, &["ts", "tsx", "css"], 20, &mut rng);
    }
    t.compute_sizes();
    t
}

pub fn fixture_state() -> AppState {
    let mut s = AppState::new(
        PathBuf::from("/Users/pavol"),
        true,
        ColorMode::Extension,
        mac_dir_stat::flywheel::disabled().clone(),
    );
    s.request_rescan = false;
    let tree = fixture_tree();
    let root = tree.root();
    s.extension_stats = tree.collect_extensions(root);
    s.cleanup_candidates = mac_dir_stat::cleanup::find_candidates(&tree, root);
    s.home = Some(PathBuf::from("/Users/pavol"));
    s.only_yours = mac_dir_stat::cleanup::find_only_yours(&tree, s.home.as_deref(), &s.cleanup_candidates);
    s.view_root = Some(root);
    s.zoom_stack = vec![root];
    s.scan_progress.files = tree.node(root).file_count as usize;
    s.scan_progress.dirs = tree.node(root).subdir_count as usize;
    s.scan_duration_secs = 4.2;
    s.tree = Some(tree);
    expand(&mut s, root);
    s
}

fn expand(s: &mut AppState, id: NodeId) {
    if let Some(t) = &mut s.tree {
        if let mac_dir_stat::scanner::tree::NodeKind::Directory { expanded, .. } =
            &mut t.node_mut(id).kind
        {
            *expanded = true;
        }
    }
}

fn inventory(s: &mut AppState) {
    s.cleanup_window_open = true;
    // One rule ticked by hand, to show the action bar with a selection.
    let ids: Vec<NodeId> = s
        .cleanup_candidates
        .iter()
        .filter(|c| c.category == "node_modules")
        .map(|c| c.node_id)
        .collect();
    s.cleanup_selected.extend(ids);
}

fn closing_account(s: &mut AppState) {
    s.cleanup_window_open = true;
    let mut a = mac_dir_stat::cleanup::Account::default();
    let moved: Vec<(PathBuf, u64)> = s
        .cleanup_candidates
        .iter()
        .filter(|c| c.category == "node_modules" || c.category == "Xcode DerivedData")
        .map(|c| (c.path.clone(), c.size))
        .collect();
    a.add(&moved);
    a.disk = Some((212 * GB + 600 * MB, 994 * GB));
    s.closing_account = Some(a);
}

// ── Rendering ───────────────────────────────────────────────────────────

struct Texture {
    size: [usize; 2],
    px: Vec<Color32>,
}

fn render(shot: &Shot, path: &Path) {
    let mut state = fixture_state();
    (shot.setup)(&mut state);
    let mut app = App::from_state(state);
    let ctx = egui::Context::default();
    ctx.set_theme(if shot.dark { ThemePreference::Dark } else { ThemePreference::Light });
    // As main.rs does: fonts must be bound a frame before they are used.
    mac_dir_stat::ui::theme::apply_theme(&ctx);
    let _ = ctx.run(Default::default(), |_| {});
    let mut frame = eframe::Frame::_new_kittest();
    let mut textures: HashMap<TextureId, Texture> = HashMap::new();
    let mut prims = Vec::new();
    // Several frames: windows size themselves and fonts load on the first.
    for i in 0..6 {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(W, H))),
            time: Some(i as f64 * 0.5),
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(PPP);
        if let Some(p) = shot.pointer {
            input.events.push(egui::Event::PointerMoved(p));
        }
        let out = ctx.run(input, |ctx| eframe::App::update(&mut app, ctx, &mut frame));
        for (id, delta) in &out.textures_delta.set {
            apply_delta(&mut textures, *id, delta);
        }
        prims = ctx.tessellate(out.shapes, out.pixels_per_point);
    }
    let (w, h) = ((W * PPP) as usize, (H * PPP) as usize);
    let mut img = vec![[0f32; 4]; w * h];
    for p in &prims {
        draw(&mut img, w, h, p, &textures);
    }
    write_png(&img, w, h, path);
}

fn apply_delta(textures: &mut HashMap<TextureId, Texture>, id: TextureId, d: &egui::epaint::ImageDelta) {
    let (size, px): ([usize; 2], Vec<Color32>) = match &d.image {
        ImageData::Color(c) => (c.size, c.pixels.clone()),
        ImageData::Font(f) => (f.size, f.srgba_pixels(None).collect()),
    };
    match d.pos {
        None => {
            textures.insert(id, Texture { size, px });
        }
        Some([x0, y0]) => {
            let t = textures.get_mut(&id).expect("partial update of unknown texture");
            for y in 0..size[1] {
                for x in 0..size[0] {
                    t.px[(y0 + y) * t.size[0] + x0 + x] = px[y * size[0] + x];
                }
            }
        }
    }
}

fn draw(img: &mut [[f32; 4]], w: usize, h: usize, p: &ClippedPrimitive, tex: &HashMap<TextureId, Texture>) {
    let Primitive::Mesh(mesh) = &p.primitive else { return };
    let Some(t) = tex.get(&mesh.texture_id) else { return };
    let clip = p.clip_rect;
    let cx0 = (clip.min.x * PPP).max(0.0) as i32;
    let cy0 = (clip.min.y * PPP).max(0.0) as i32;
    let cx1 = ((clip.max.x * PPP).ceil() as i32).min(w as i32);
    let cy1 = ((clip.max.y * PPP).ceil() as i32).min(h as i32);
    for tri in mesh.indices.chunks_exact(3) {
        let v = [
            &mesh.vertices[tri[0] as usize],
            &mesh.vertices[tri[1] as usize],
            &mesh.vertices[tri[2] as usize],
        ];
        let pt: Vec<(f32, f32)> = v.iter().map(|v| (v.pos.x * PPP, v.pos.y * PPP)).collect();
        let area = edge(pt[0], pt[1], pt[2]);
        if area.abs() < 1e-6 {
            continue;
        }
        let minx = pt.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor().max(cx0 as f32) as i32;
        let maxx = pt.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil().min(cx1 as f32) as i32;
        let miny = pt.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor().max(cy0 as f32) as i32;
        let maxy = pt.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil().min(cy1 as f32) as i32;
        for y in miny..maxy {
            for x in minx..maxx {
                let c = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = edge(pt[1], pt[2], c) / area;
                let w1 = edge(pt[2], pt[0], c) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                    continue;
                }
                let mut col = [0f32; 4];
                for k in 0..4 {
                    col[k] = (v[0].color[k] as f32 * w0 + v[1].color[k] as f32 * w1 + v[2].color[k] as f32 * w2) / 255.0;
                }
                let u = v[0].uv.x * w0 + v[1].uv.x * w1 + v[2].uv.x * w2;
                let vv = v[0].uv.y * w0 + v[1].uv.y * w1 + v[2].uv.y * w2;
                let tx = ((u * t.size[0] as f32) as usize).min(t.size[0] - 1);
                let ty = ((vv * t.size[1] as f32) as usize).min(t.size[1] - 1);
                let s = t.px[ty * t.size[0] + tx];
                for k in 0..4 {
                    col[k] *= s[k] as f32 / 255.0;
                }
                // Premultiplied "over".
                let d = &mut img[y as usize * w + x as usize];
                for k in 0..4 {
                    d[k] = col[k] + d[k] * (1.0 - col[3]);
                }
            }
        }
    }
}

fn edge(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn write_png(img: &[[f32; 4]], w: usize, h: usize, path: &Path) {
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in img {
        for k in 0..3 {
            ppm.push((p[k].clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    let tmp = path.with_extension("ppm");
    std::fs::write(&tmp, ppm).unwrap();
    // Downscale to 1x points so the PNGs stay small in the repo.
    let ok = std::process::Command::new("convert")
        .arg(&tmp)
        .args(["-filter", "Lanczos", "-resize", "50%", "-strip", "-define", "png:compression-level=9"])
        .arg(path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&tmp);
    assert!(ok, "ImageMagick `convert` is needed to write PNGs");
}
