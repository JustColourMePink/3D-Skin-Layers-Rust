// 3D Skin Layers for Horizon: the shared core.
//
// A port of tr7zw's 3D Skin Layers (https://github.com/tr7zw/3d-Skin-Layers),
// Copyright (c) tr7zw, 2021, under the tr7zw Protective License: free to use,
// modify and compile, but not for a commercial advantage or monetary
// compensation. See LICENSE-skinlayers3d.txt next to this bundle.
//
// `horizon_client_render::skin_layers_3d_core`: what both crates need. The
// voxel builder (`SolidPixelWrapper`, `CustomizableCube`), the settings the
// game sets, the decoded skins (mirrored from `skins::Downloader`, which the
// block-entity and item renderers cannot reach) and the 3D player-head models
// those renderers draw.

use crate::block_entity_render::ModelPart;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};

/// A prepared skin image is always this many texels a side.
pub const SKIN: i32 = 64;
/// Decoded skins kept for the heads (64×64 RGBA: 16 KiB each).
const IMAGES_KEPT: usize = 512;
/// The most head models ever built: each is kept for the rest of the run
/// (the renderers borrow them for a frame with no owner to hand back).
const HEADS_EVER: usize = 2048;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// `dev.tr7zw.skinlayers.versionless.config.Config`, minus the Java renderer's
/// workarounds (compatibility, Iris and Sodium modes) Horizon has no use for.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub hat: bool,
    pub jacket: bool,
    pub left_sleeve: bool,
    pub right_sleeve: bool,
    pub left_pants: bool,
    pub right_pants: bool,
    /// X/Z scale of the limbs' layer (and Z of the torso's).
    pub base_voxel_size: f32,
    /// X scale of the torso's layer.
    pub body_voxel_width_size: f32,
    /// Uniform scale of the hat.
    pub head_voxel_size: f32,
    /// X/Z scale of the first-person sleeve.
    pub first_person_pixel_scaling: f32,
    /// Blocks: further players show the flat vanilla layer.
    pub lod_distance: i32,
    /// Placed player heads in 3D.
    pub skulls: bool,
    /// Player head items (held, in a slot, dropped, worn) in 3D.
    pub skull_items: bool,
    /// Uniform scale of a player head's hat.
    pub skull_voxel_size: f32,
    /// The original's "Fast Render".
    pub fast_render: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hat: true,
            jacket: true,
            left_sleeve: true,
            right_sleeve: true,
            left_pants: true,
            right_pants: true,
            base_voxel_size: 1.15,
            body_voxel_width_size: 1.05,
            head_voxel_size: 1.18,
            first_person_pixel_scaling: 1.1,
            lod_distance: 14,
            skulls: true,
            skull_items: true,
            skull_voxel_size: 1.1,
            fast_render: true,
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Self {
        let mut c = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            let flag = value == "true";
            let float = value.parse::<f32>().ok().filter(|v| v.is_finite());
            match key {
                "hat" => c.hat = flag,
                "jacket" => c.jacket = flag,
                "left_sleeve" => c.left_sleeve = flag,
                "right_sleeve" => c.right_sleeve = flag,
                "left_pants" => c.left_pants = flag,
                "right_pants" => c.right_pants = flag,
                "skulls" => c.skulls = flag,
                "skull_items" => c.skull_items = flag,
                "fast_render" => c.fast_render = flag,
                "base_voxel_size" => c.base_voxel_size = float.unwrap_or(c.base_voxel_size),
                "body_voxel_width_size" => {
                    c.body_voxel_width_size = float.unwrap_or(c.body_voxel_width_size)
                }
                "head_voxel_size" => c.head_voxel_size = float.unwrap_or(c.head_voxel_size),
                "first_person_pixel_scaling" => {
                    c.first_person_pixel_scaling = float.unwrap_or(c.first_person_pixel_scaling)
                }
                "skull_voxel_size" => c.skull_voxel_size = float.unwrap_or(c.skull_voxel_size),
                "lod_distance" => {
                    c.lod_distance = value.parse().unwrap_or(c.lod_distance).clamp(0, 512)
                }
                _ => {}
            }
        }
        for size in [
            &mut c.base_voxel_size,
            &mut c.body_voxel_width_size,
            &mut c.head_voxel_size,
            &mut c.first_person_pixel_scaling,
            &mut c.skull_voxel_size,
        ] {
            *size = size.clamp(0.5, 2.0);
        }
        c
    }

    pub fn write(&self) -> String {
        format!(
            "# 3D Skin Layers for Horizon\n\
             hat={}\njacket={}\nleft_sleeve={}\nright_sleeve={}\nleft_pants={}\nright_pants={}\n\
             base_voxel_size={}\nbody_voxel_width_size={}\nhead_voxel_size={}\n\
             first_person_pixel_scaling={}\nlod_distance={}\n\
             skulls={}\nskull_items={}\nskull_voxel_size={}\nfast_render={}\n",
            self.hat,
            self.jacket,
            self.left_sleeve,
            self.right_sleeve,
            self.left_pants,
            self.right_pants,
            self.base_voxel_size,
            self.body_voxel_width_size,
            self.head_voxel_size,
            self.first_person_pixel_scaling,
            self.lod_distance,
            self.skulls,
            self.skull_items,
            self.skull_voxel_size,
            self.fast_render,
        )
    }

    /// Whether the player-model part `name` is drawn in 3D.
    pub fn enabled(&self, name: &str) -> bool {
        match name {
            "hat" => self.hat,
            "jacket" => self.jacket,
            "left_sleeve" => self.left_sleeve,
            "right_sleeve" => self.right_sleeve,
            "left_pants" => self.left_pants,
            "right_pants" => self.right_pants,
            _ => false,
        }
    }
}

/// The settings and their revision, which changes with every change.
static CONFIG: RwLock<Option<(Config, u64)>> = RwLock::new(None);

/// The settings now (the defaults until the game loads the player's).
pub fn config() -> Config {
    let guard = CONFIG.read().unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.as_ref().map(|(c, _)| c.clone()).unwrap_or_default()
}

fn revision() -> u64 {
    let guard = CONFIG.read().unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.as_ref().map_or(0, |(_, r)| *r)
}

/// New settings: every model built under the old ones is built again.
pub fn set_config(config: Config) {
    let mut guard = CONFIG.write().unwrap_or_else(std::sync::PoisonError::into_inner);
    let revision = guard.as_ref().map_or(0, |(_, r)| r + 1);
    *guard = Some((config, revision));
}

// ---------------------------------------------------------------------------
// The decoded skins
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Images {
    by_location: HashMap<String, Arc<[u8]>>,
    order: VecDeque<String>,
}

static IMAGES: Mutex<Option<Images>> = Mutex::new(None);

/// `skins::Downloader::remember`: a skin decoded under `location`.
pub fn remember(location: &str, rgba: &[u8]) {
    if rgba.len() < (SKIN * SKIN * 4) as usize {
        return;
    }
    {
        let mut guard = IMAGES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let images = guard.get_or_insert_with(Images::default);
        if !images.by_location.contains_key(location) {
            images.order.push_back(location.to_owned());
            while images.order.len() > IMAGES_KEPT {
                if let Some(old) = images.order.pop_front() {
                    images.by_location.remove(&old);
                }
            }
        }
        images.by_location.insert(location.to_owned(), Arc::from(rgba));
    }
    // A skin decoded again (a new download under an old slot) builds again.
    // IMAGES is let go first: `head_model` takes HEADS, then IMAGES.
    let mut heads = HEADS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(heads) = heads.as_mut() {
        heads.models.retain(|(loc, _), _| loc != location);
    }
}

fn image(location: &str) -> Option<Arc<[u8]>> {
    let guard = IMAGES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.as_ref()?.by_location.get(location).cloned()
}

/// Every skin location in memory, for the game to find their atlas sprites.
pub fn locations() -> Vec<String> {
    let guard = IMAGES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    guard
        .as_ref()
        .map(|images| images.order.iter().cloned().collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Which skin an atlas sprite is
// ---------------------------------------------------------------------------

/// The block-entity and item renderers draw a head with the skin's atlas
/// sprite, not its name: the game hands over which sprite is which skin.
static SPRITES: RwLock<Option<HashMap<[u32; 4], String>>> = RwLock::new(None);

/// A sprite's key: its atlas rectangle, to the bit.
pub fn sprite_key(rect: [f32; 4]) -> [u32; 4] {
    rect.map(f32::to_bits)
}

/// The game's map of skin sprites (`sprite_key` → skin location).
pub fn set_sprites(sprites: HashMap<[u32; 4], String>) {
    *SPRITES.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(sprites);
}

fn skin_of(rect: [f32; 4]) -> Option<String> {
    let guard = SPRITES.read().unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.as_ref()?.get(&sprite_key(rect)).cloned()
}

/// Whether `part` is a player head's root: a `head` holding a `hat`.
fn is_player_head(part: &ModelPart) -> bool {
    part.children.iter().any(|(name, head)| {
        name == "head" && head.children.iter().any(|(name, _)| name == "hat")
    })
}

/// What to draw instead of `part` (named `name`, painted with the atlas
/// sprite `rect`): a player head's model with its hat in 3D, when `part` is a
/// player head's root, `rect` is a skin in memory and 3D heads are on for
/// blocks (`item` false) or items. `None`: draw `part` itself.
pub fn head_for<'a>(part: &'a ModelPart, name: &str, rect: [f32; 4], item: bool) -> Option<&'a ModelPart> {
    if name != "root" || !is_player_head(part) {
        return None;
    }
    let skin = skin_of(rect)?;
    let model = head_model(part, Some(&skin), item);
    // Our own model, drawn by the hook, comes back here as itself.
    (!std::ptr::eq(model, part)).then_some(model)
}

// ---------------------------------------------------------------------------
// Player heads (SkullBlockEntityRendererMixin, CustomHeadLayerMixin, items)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Heads {
    revision: u64,
    built: usize,
    /// By (skin location, `skull_voxel_size` bits).
    models: HashMap<(String, u32), &'static ModelPart>,
}

static HEADS: Mutex<Option<Heads>> = Mutex::new(None);

/// The player-head model to draw for `skin`: `vanilla` with its hat in 3D
/// when that is on for blocks (`item` false) or items (`item` true) and the
/// skin is in memory, else `vanilla` itself.
pub fn head_model<'a>(vanilla: &'a ModelPart, skin: Option<&str>, item: bool) -> &'a ModelPart {
    let Some(skin) = skin else {
        return vanilla;
    };
    let config = config();
    if !(if item { config.skull_items } else { config.skulls }) {
        return vanilla;
    }
    let key = (skin.to_owned(), config.skull_voxel_size.to_bits());
    let mut guard = HEADS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let heads = guard.get_or_insert_with(Heads::default);
    let now = revision();
    if heads.revision != now {
        heads.revision = now;
        heads.models.clear();
    }
    if let Some(model) = heads.models.get(&key) {
        return model;
    }
    if heads.built >= HEADS_EVER {
        return vanilla;
    }
    let Some(rgba) = image(skin) else {
        return vanilla;
    };
    let mut model = vanilla.clone();
    let texture = Texture(&rgba);
    replace_head(&mut model, &texture, &config);
    heads.built += 1;
    let model: &'static ModelPart = Box::leak(Box::new(model));
    heads.models.insert(key, model);
    model
}

fn replace_head(part: &mut ModelPart, texture: &Texture<'_>, config: &Config) {
    for (name, child) in &mut part.children {
        if name == "hat" {
            let layer = Layer::of("hat", false).expect("the hat is a layer");
            child.polygons = layer
                .quads(texture, config, Mode::Skull)
                .into_iter()
                .map(|q| q.vertices)
                .collect();
        }
        replace_head(child, texture, config);
    }
}

// ---------------------------------------------------------------------------
// The voxel builder (SolidPixelWrapper + CustomizableCube)
// ---------------------------------------------------------------------------

/// Where a layer is drawn, which picks `OffsetProvider`'s scale.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    ThirdPerson,
    FirstPerson,
    Skull,
}

/// One face of a voxel: its normal and its corners (x, y, z in model pixels,
/// u, v on the skin), in vanilla's winding.
#[derive(Clone, Debug, PartialEq)]
pub struct Quad {
    pub normal: [f32; 3],
    pub vertices: [[f32; 5]; 4],
}

/// The voxel faces of player-model part `name` (`hat`, `jacket`, sleeves,
/// pants) for the 64×64 RGBA `rgba`, or `None` when `name` isn't one of them
/// or the image is not a full skin.
pub fn layer_quads(name: &str, slim: bool, rgba: &[u8], config: &Config, mode: Mode) -> Option<Vec<Quad>> {
    if rgba.len() < (SKIN * SKIN * 4) as usize {
        return None;
    }
    let layer = Layer::of(name, slim)?;
    Some(layer.quads(&Texture(rgba), config, mode))
}

/// The decoded skin: 64×64 RGBA, rows top to bottom.
struct Texture<'a>(&'a [u8]);

impl Texture<'_> {
    fn alpha(&self, uv: Uv) -> u8 {
        if uv.u < 0 || uv.v < 0 || uv.u >= SKIN || uv.v >= SKIN {
            return 0;
        }
        self.0[((uv.v * SKIN + uv.u) * 4 + 3) as usize]
    }
    /// `TextureData.isPresent`: any alpha at all.
    fn present(&self, uv: Uv) -> bool {
        self.alpha(uv) != 0
    }
    /// `TextureData.isSolid`: fully opaque.
    fn solid(&self, uv: Uv) -> bool {
        self.alpha(uv) == 255
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Dir {
    /// `Direction.values()`: DOWN, UP, NORTH, SOUTH, WEST, EAST.
    const ALL: [Dir; 6] = [Dir::Down, Dir::Up, Dir::North, Dir::South, Dir::West, Dir::East];

    fn step(self) -> [i32; 3] {
        match self {
            Dir::Down => [0, -1, 0],
            Dir::Up => [0, 1, 0],
            Dir::North => [0, 0, -1],
            Dir::South => [0, 0, 1],
            Dir::West => [-1, 0, 0],
            Dir::East => [1, 0, 0],
        }
    }
    /// 0 = X, 1 = Y, 2 = Z.
    fn axis(self) -> usize {
        match self {
            Dir::West | Dir::East => 0,
            Dir::Down | Dir::Up => 1,
            Dir::North | Dir::South => 2,
        }
    }
    fn opposite(self) -> Dir {
        match self {
            Dir::Down => Dir::Up,
            Dir::Up => Dir::Down,
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::West => Dir::East,
            Dir::East => Dir::West,
        }
    }
    /// +1 for the positive directions, -1 for the negative ones.
    fn dir_step(self) -> f32 {
        let s = self.step();
        (s[0] + s[1] + s[2]) as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Uv {
    u: i32,
    v: i32,
}

const fn uv(u: i32, v: i32) -> Uv {
    Uv { u, v }
}

#[derive(Clone, Copy, Debug)]
struct Dims {
    w: i32,
    h: i32,
    d: i32,
}

type Voxel = [i32; 3];

fn add(a: Voxel, b: [i32; 3]) -> Voxel {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Voxel, b: [i32; 3]) -> Voxel {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `getSizeUV`: a face's size on the texture.
fn size_uv(d: Dims, face: Dir) -> Uv {
    match face {
        Dir::Down | Dir::Up => uv(d.w, d.d),
        Dir::North | Dir::South => uv(d.w, d.h),
        Dir::West | Dir::East => uv(d.d, d.h),
    }
}

/// `getOnTextureUV`: `local` on `face` of the box unwrapped at `base`.
fn on_texture(base: Uv, local: Uv, d: Dims, face: Dir) -> Uv {
    let (u, v) = match face {
        Dir::Down => (base.u + d.d, base.v),
        Dir::Up => (base.u + d.w + d.d, base.v),
        Dir::North => (base.u + d.d, base.v + d.d),
        Dir::South => (base.u + d.d + d.w + d.d, base.v + d.d),
        Dir::West => (base.u, base.v + d.d),
        Dir::East => (base.u + d.d + d.w, base.v + d.d),
    };
    uv(u + local.u, v + local.v)
}

/// `UVtoXYZ`: the voxel a face's texel sits in.
fn to_voxel(local: Uv, d: Dims, face: Dir) -> Voxel {
    let Uv { u, v } = local;
    match face {
        Dir::Down => [u, 0, d.d - 1 - v],
        Dir::Up => [u, d.h - 1, d.d - 1 - v],
        Dir::North => [u, v, 0],
        Dir::South => [d.w - 1 - u, v, d.d - 1],
        Dir::West => [0, v, d.d - 1 - u],
        Dir::East => [d.w - 1, v, u],
    }
}

/// `XYZtoUV`: where a voxel lands on a face's texel grid (maybe off it).
fn to_face(p: Voxel, d: Dims, face: Dir) -> Uv {
    let [x, y, z] = p;
    match face {
        Dir::Down | Dir::Up => uv(x, d.d - 1 - z),
        Dir::North => uv(x, y),
        Dir::South => uv(d.w - 1 - x, y),
        Dir::West => uv(d.d - 1 - z, y),
        Dir::East => uv(z, y),
    }
}

fn on_face(p: Uv, size: Uv) -> bool {
    p.u >= 0 && p.u < size.u && p.v >= 0 && p.v < size.v
}

/// One voxel: its corner, texel, hidden faces and bevelled corners.
struct Cube {
    at: [f32; 3],
    texel: Uv,
    hidden: Vec<Dir>,
    corners: Vec<[Dir; 2]>,
}

/// `SolidPixelWrapper.wrapBox`: the voxels of a `d`-sized box unwrapped at `base`.
fn wrap_box(texture: &Texture<'_>, d: Dims, base: Uv, top_pivot: bool, offset: f32, fast: bool) -> Vec<Cube> {
    let origin = [
        -(d.w as f32) / 2.0,
        if top_pivot { offset } else { -(d.h as f32) + offset },
        -(d.d as f32) / 2.0,
    ];
    let mut cubes = Vec::new();
    for face in Dir::ALL {
        let size = size_uv(d, face);
        for u in 0..size.u {
            for v in 0..size.v {
                if let Some(cube) = pixel(texture, origin, face, d, uv(u, v), base, size, fast) {
                    cubes.push(cube);
                }
            }
        }
    }
    cubes
}

/// `SolidPixelWrapper.addPixel`.
#[allow(clippy::too_many_arguments)]
fn pixel(
    texture: &Texture<'_>,
    origin: [f32; 3],
    face: Dir,
    d: Dims,
    local: Uv,
    base: Uv,
    size: Uv,
    fast: bool,
) -> Option<Cube> {
    let texel = on_texture(base, local, d, face);
    if !texture.present(texel) {
        return None;
    }
    let voxel = to_voxel(local, d, face);
    let solid = texture.solid(texel);
    // A neighbour hides a side when it is there, and opaque when this one is.
    let matches = |t: Uv| texture.present(t) && (!solid || texture.solid(t));
    let mut hidden = Vec::new();
    let mut corners = Vec::new();
    let mut on_edge = false;
    let mut joined = false;
    for n in Dir::ALL {
        if n.axis() == face.axis() {
            continue;
        }
        let next = add(voxel, n.step());
        let next_uv = to_face(next, d, face);
        if on_face(next_uv, size) {
            if texture.present(on_texture(base, next_uv, d, face)) {
                if matches(on_texture(base, next_uv, d, face)) {
                    hidden.push(n);
                }
            } else {
                // A gap of one texel: the voxel two along, on the next face over.
                let beyond = add(next, n.step());
                if !on_face(to_face(beyond, d, face), size) {
                    let there = to_face(beyond, d, n);
                    if matches(on_texture(base, there, d, n)) {
                        hidden.push(n);
                    }
                }
            }
        } else {
            on_edge = true;
            let there = to_face(voxel, d, n);
            if texture.present(on_texture(base, there, d, n)) {
                joined = true;
                hidden.push(n);
                corners.push([face.opposite(), n]);
            } else {
                let under = to_face(sub(voxel, face.step()), d, n);
                if texture.present(on_texture(base, under, d, n)) {
                    joined = true;
                }
            }
        }
    }
    // The inward face sits against the body. Kept only on an edge voxel with
    // no neighbour round the corner, where it can show from the side; Fast
    // Render drops it there too. (The Java code adds `face` itself here, but
    // with Horizon's face convention that is the outward face: tested in game,
    // it hollowed the layer out.)
    if !on_edge || joined || fast {
        hidden.push(face.opposite());
    }
    Some(Cube {
        at: [
            origin[0] + voxel[0] as f32,
            origin[1] + voxel[1] as f32,
            origin[2] + voxel[2] as f32,
        ],
        texel,
        hidden,
        corners,
    })
}

/// `CustomizableCube`: a unit cube's faces, in vanilla's order and winding,
/// textured with its one texel, `place`d into the part's space.
fn cube_quads(cube: &Cube, place: &dyn Fn([f32; 3]) -> [f32; 3], out: &mut Vec<Quad>) {
    let [x0, y0, z0] = cube.at;
    let (x1, y1, z1) = (x0 + 1.0, y0 + 1.0, z0 + 1.0);
    let v = [
        [x0, y0, z0],
        [x1, y0, z0],
        [x1, y1, z0],
        [x0, y1, z0],
        [x0, y0, z1],
        [x1, y0, z1],
        [x1, y1, z1],
        [x0, y1, z1],
    ];
    // Per axis, the corner pair whose bevel that axis's faces take: the first
    // axis (X, Y, Z) the pair doesn't lie along; a later pair wins.
    let mut bevel: [Option<[Dir; 2]>; 3] = [None; 3];
    for pair in &cube.corners {
        if let Some(axis) = (0..3).find(|&a| pair.iter().all(|d| d.axis() != a)) {
            bevel[axis] = Some(*pair);
        }
    }
    let size = SKIN as f32;
    let (u0, v0) = (cube.texel.u as f32 / size, cube.texel.v as f32 / size);
    let (u1, v1) = ((cube.texel.u + 1) as f32 / size, (cube.texel.v + 1) as f32 / size);
    let faces: [(Dir, [usize; 4]); 6] = [
        (Dir::Down, [5, 4, 0, 1]),
        (Dir::Up, [2, 3, 7, 6]),
        (Dir::North, [1, 0, 3, 2]),
        (Dir::South, [4, 5, 6, 7]),
        (Dir::West, [0, 4, 7, 3]),
        (Dir::East, [5, 1, 2, 6]),
    ];
    for (dir, corners) in faces {
        if cube.hidden.contains(&dir) {
            continue;
        }
        let mut quad = corners.map(|i| v[i]);
        if let Some(pair) = bevel[dir.axis()] {
            quad = remove_corner(quad, pair);
        }
        let uvs = [(u1, v0), (u0, v0), (u0, v1), (u1, v1)];
        let s = dir.step();
        out.push(Quad {
            normal: [s[0] as f32, s[1] as f32, s[2] as f32],
            vertices: std::array::from_fn(|k| {
                let p = place(quad[k]);
                [p[0], p[1], p[2], uvs[k].0, uvs[k].1]
            }),
        });
    }
}

/// `CustomizableCube.removeCornerVertex`: drops the vertex furthest along
/// `pair[0]` (then `pair[1]`), making the quad a triangle.
fn remove_corner(quad: [[f32; 3]; 4], pair: [Dir; 2]) -> [[f32; 3]; 4] {
    let further = |a: [f32; 3], b: [f32; 3]| {
        for dir in pair {
            let axis = dir.axis();
            let along = (a[axis] - b[axis]) * dir.dir_step();
            if along > 0.0 {
                return a;
            }
            if along < 0.0 {
                return b;
            }
        }
        a
    };
    let mut gone = 0;
    for i in 1..4 {
        if further(quad[gone], quad[i]) != quad[gone] {
            gone = i;
        }
    }
    let mut out = [[0.0; 3]; 4];
    let mut n = 0;
    for (i, p) in quad.iter().enumerate() {
        if i != gone {
            out[n] = *p;
            n += 1;
        }
    }
    out[3] = out[2];
    out
}

/// `OffsetProvider`'s shapes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Head,
    Body,
    Legs,
    Arms,
    ArmsSlim,
}

impl Shape {
    /// `Shape.yOffsetMagicValue`.
    fn y_offset(self) -> f32 {
        match self {
            Shape::Head => 0.0,
            Shape::Body | Shape::Legs => -0.2,
            Shape::Arms | Shape::ArmsSlim => -0.1,
        }
    }
}

/// One outer-layer part: `SkinUtil.setup3dLayers`' mesh and `OffsetProvider`'s placement.
struct Layer {
    dims: Dims,
    base: Uv,
    top_pivot: bool,
    offset: f32,
    shape: Shape,
    right: bool,
}

impl Layer {
    fn of(name: &str, slim: bool) -> Option<Self> {
        let arm = |base: Uv, right: bool| Layer {
            dims: Dims { w: if slim { 3 } else { 4 }, h: 12, d: 4 },
            base,
            top_pivot: true,
            offset: -2.0,
            shape: if slim { Shape::ArmsSlim } else { Shape::Arms },
            right,
        };
        let leg = |base: Uv| Layer {
            dims: Dims { w: 4, h: 12, d: 4 },
            base,
            top_pivot: true,
            offset: 0.0,
            shape: Shape::Legs,
            right: false,
        };
        Some(match name {
            "hat" => Layer {
                dims: Dims { w: 8, h: 8, d: 8 },
                base: uv(32, 0),
                top_pivot: false,
                offset: 0.6,
                shape: Shape::Head,
                right: false,
            },
            "jacket" => Layer {
                dims: Dims { w: 8, h: 12, d: 4 },
                base: uv(16, 32),
                top_pivot: true,
                offset: 0.0,
                shape: Shape::Body,
                right: false,
            },
            "left_sleeve" => arm(uv(48, 48), false),
            "right_sleeve" => arm(uv(40, 32), true),
            "left_pants" => leg(uv(0, 48)),
            "right_pants" => leg(uv(0, 32)),
            _ => return None,
        })
    }

    fn quads(&self, texture: &Texture<'_>, config: &Config, mode: Mode) -> Vec<Quad> {
        let cubes = wrap_box(texture, self.dims, self.base, self.top_pivot, self.offset, config.fast_render);
        let place = self.placement(config, mode);
        let mut out = Vec::with_capacity(cubes.len() * 3);
        for cube in &cubes {
            cube_quads(cube, &place, &mut out);
        }
        out
    }

    /// `OffsetProvider.createVanilla`'s pose, baked into the vertices. Units are
    /// model pixels; the PoseStack's 0.25 and 0.04 blocks are 4 and 0.64 pixels.
    fn placement(&self, config: &Config, mode: Mode) -> Box<dyn Fn([f32; 3]) -> [f32; 3]> {
        if self.shape == Shape::Head {
            let s = if mode == Mode::Skull {
                config.skull_voxel_size
            } else {
                config.head_voxel_size
            };
            return Box::new(move |p| [p[0] * s, (p[1] + 4.0 - 0.64) * s - 4.0, p[2] * s]);
        }
        let (mut x, z) = if mode == Mode::FirstPerson {
            (config.first_person_pixel_scaling, config.first_person_pixel_scaling)
        } else {
            (config.base_voxel_size, config.base_voxel_size)
        };
        if self.shape == Shape::Body {
            x = config.body_voxel_width_size;
        }
        let mut dx = match self.shape {
            Shape::Arms => 0.998,
            Shape::ArmsSlim => 0.499,
            _ => 0.0,
        };
        if self.right {
            dx = -dx;
        }
        let dy = self.shape.y_offset();
        Box::new(move |p| [(p[0] + dx) * x, (p[1] + dy) * 1.035, p[2] * z])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin(painted: &[(i32, i32, u8)]) -> Vec<u8> {
        let mut image = vec![0u8; (SKIN * SKIN * 4) as usize];
        for &(u, v, a) in painted {
            let i = ((v * SKIN + u) * 4) as usize;
            image[i..i + 4].copy_from_slice(&[255, 255, 255, a]);
        }
        image
    }

    #[test]
    fn uv_round_trips_on_every_face() {
        let d = Dims { w: 4, h: 12, d: 4 };
        for face in Dir::ALL {
            let size = size_uv(d, face);
            for u in 0..size.u {
                for v in 0..size.v {
                    let p = to_voxel(uv(u, v), d, face);
                    assert_eq!(to_face(p, d, face), uv(u, v), "{face:?} {u},{v}");
                }
            }
        }
    }

    #[test]
    fn a_blank_layer_has_no_voxels() {
        let image = skin(&[]);
        let quads = layer_quads("hat", false, &image, &Config::default(), Mode::ThirdPerson);
        assert_eq!(quads, Some(Vec::new()));
    }

    #[test]
    fn one_texel_in_the_middle_of_a_face_is_one_voxel() {
        // The hat's front (NORTH) starts at (40, 8); texel (3, 3) of it.
        let image = skin(&[(43, 11, 255)]);
        let cubes = wrap_box(&Texture(&image), Dims { w: 8, h: 8, d: 8 }, uv(32, 0), false, 0.6, false);
        assert_eq!(cubes.len(), 1);
        let cube = &cubes[0];
        assert_eq!(cube.texel, uv(43, 11));
        // Not on an edge: the inward face is hidden, nothing else.
        assert_eq!(cube.hidden, vec![Dir::South]);
        assert!(cube.corners.is_empty());
    }

    #[test]
    fn fast_render_keeps_the_outward_face() {
        // Top-left corner texel of the hat's front: on two edges, no neighbours.
        let image = skin(&[(40, 8, 255)]);
        let d = Dims { w: 8, h: 8, d: 8 };
        let slow = wrap_box(&Texture(&image), d, uv(32, 0), false, 0.6, false);
        let fast = wrap_box(&Texture(&image), d, uv(32, 0), false, 0.6, true);
        assert!(!slow[0].hidden.contains(&Dir::South), "an open edge keeps its inward face");
        assert!(fast[0].hidden.contains(&Dir::South), "fast render drops it");
        assert!(!fast[0].hidden.contains(&Dir::North), "the outward face always draws");
    }

    #[test]
    fn neighbours_hide_the_faces_between_them() {
        let image = skin(&[(43, 11, 255), (44, 11, 255)]);
        let cubes = wrap_box(&Texture(&image), Dims { w: 8, h: 8, d: 8 }, uv(32, 0), false, 0.6, false);
        assert_eq!(cubes.len(), 2);
        assert!(cubes[0].hidden.contains(&Dir::East));
        assert!(cubes[1].hidden.contains(&Dir::West));
    }

    #[test]
    fn a_bevelled_face_is_a_triangle() {
        let quad = [[1.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let out = remove_corner(quad, [Dir::East, Dir::South]);
        assert_eq!(out[3], out[2]);
        assert!(!out.contains(&[1.0, 0.0, 1.0]));
    }

    #[test]
    fn skulls_use_their_own_voxel_size() {
        let image = skin(&[(43, 11, 255)]);
        let mut config = Config::default();
        config.skull_voxel_size = 1.0;
        config.head_voxel_size = 2.0;
        let skull = layer_quads("hat", false, &image, &config, Mode::Skull).unwrap();
        let worn = layer_quads("hat", false, &image, &config, Mode::ThirdPerson).unwrap();
        // At scale 1 the voxel stays where it was built: x from -1 to 0.
        let xs = |q: &[Quad]| q.iter().flat_map(|q| q.vertices.iter().map(|v| v[0])).fold(f32::MIN, f32::max);
        assert!((xs(&skull) - 0.0).abs() < 1e-5);
        assert!((xs(&worn) - 0.0).abs() < 1e-5, "x 0 scales to 0");
        assert_ne!(skull, worn);
    }

    fn part(children: Vec<(&str, ModelPart)>) -> ModelPart {
        ModelPart {
            pose: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            polygons: Vec::new(),
            children: children.into_iter().map(|(n, p)| (n.to_owned(), p)).collect(),
        }
    }

    #[test]
    fn a_player_head_with_a_known_skin_swaps_once() {
        let head = part(vec![("head", part(vec![("hat", part(vec![]))]))]);
        let chest = part(vec![("lid", part(vec![]))]);
        let rect = [0.25, 0.5, 0.375, 0.625];
        remember("skins/test_head", &skin(&[(43, 11, 255)]));
        set_sprites(HashMap::from([(sprite_key(rect), "skins/test_head".to_owned())]));
        assert!(head_for(&chest, "root", rect, false).is_none(), "not a head");
        assert!(head_for(&head, "hat", rect, false).is_none(), "only at the root");
        assert!(head_for(&head, "root", [0.0; 4], false).is_none(), "an unknown sprite");
        let swapped = head_for(&head, "root", rect, false).expect("a 3D head");
        let hat = &swapped.children[0].1.children[0].1;
        assert!(!hat.polygons.is_empty(), "the hat has voxels");
        // Drawn by the hook, the swapped model is left alone: no recursion.
        assert!(head_for(swapped, "root", rect, false).is_none());
    }

    #[test]
    fn config_round_trips() {
        let mut c = Config::default();
        c.hat = false;
        c.lod_distance = 32;
        c.head_voxel_size = 1.25;
        c.skull_items = false;
        c.skull_voxel_size = 1.3;
        assert_eq!(Config::parse(&c.write()), c);
    }
}
