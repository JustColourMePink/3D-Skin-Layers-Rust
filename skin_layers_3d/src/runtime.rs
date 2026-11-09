// 3D Skin Layers for Horizon: the game's half.
//
// A port of tr7zw's 3D Skin Layers (https://github.com/tr7zw/3d-Skin-Layers),
// Copyright (c) tr7zw, 2021, under the tr7zw Protective License: free to use,
// modify and compile, but not for a commercial advantage or monetary
// compensation. See LICENSE-skinlayers3d.txt next to this bundle.
//
// The player's outer skin layer (hat, jacket, sleeves, trousers) becomes one
// 1×1×1 voxel per painted texel (`horizon_client_render::skin_layers_3d_core`
// builds them). The models go into Horizon's own 3D skin layer slot
// (`Mob::skin_3d`), so the CPU and GPU pose paths draw them like any model.
// Here: the settings file and page, the player models and their level of
// detail, and the first-person arm.
//
// This module is a child of `crate::game`: it reads the game's own fields.

use super::Game;
use horizon_client::mobs::{MobType, Part, Polygon};
use horizon_client_render::skin_layers_3d_core::{self as core, Config, Mode};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

pub(crate) const ID: &str = "skin_layers_3d";
const TITLE: &str = "3D Skin Layers";
const VERSION: &str = "0.2.1";
/// The most skins kept built at once before the cache starts over.
const CACHE_LIMIT: usize = 256;
/// `SkinLayersModBase.hideHeadLayers`: a player wearing one of these keeps
/// the flat hat, which the head covers.
const HIDE_HAT: [&str; 5] = [
    "minecraft:zombie_head",
    "minecraft:creeper_head",
    "minecraft:dragon_head",
    "minecraft:skeleton_skull",
    "minecraft:wither_skeleton_skull",
];

// ---------------------------------------------------------------------------
// State: the built player models
// ---------------------------------------------------------------------------

#[derive(Default)]
struct State {
    loaded: bool,
    /// The core's config revision these models were built under.
    config: Config,
    /// Third-person models by skin texture: `[wide, wide flat hat, slim, slim flat hat]`.
    models: HashMap<String, [Option<Arc<Part>>; 4]>,
    /// First-person arms by skin texture: `[wide right, wide left, slim right, slim left]`.
    arms: HashMap<String, [Option<Arc<Part>>; 4]>,
    /// Frames since the head renderers' sprite map was made.
    sprites_age: u32,
}

/// How many frames the skins' atlas sprites are looked up again after: a new
/// skin's head turns 3D this long after it decodes, at most.
const SPRITES_EVERY: u32 = 30;

static STATE: Mutex<Option<State>> = Mutex::new(None);

struct Guard(MutexGuard<'static, Option<State>>);

impl std::ops::Deref for Guard {
    type Target = State;
    fn deref(&self) -> &State {
        self.0.as_ref().expect("initialised in state()")
    }
}

impl std::ops::DerefMut for Guard {
    fn deref_mut(&mut self) -> &mut State {
        self.0.as_mut().expect("initialised in state()")
    }
}

/// The state, the player's settings read from the mod's folder (and handed
/// to the core) the first time.
fn state(game: &Game<'_>) -> Guard {
    let mut guard = STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let s = guard.get_or_insert_with(State::default);
    if !s.loaded {
        s.loaded = true;
        let path = game.native_data_dir(ID).join("config.txt");
        if let Ok(text) = std::fs::read_to_string(&path) {
            s.config = Config::parse(&text);
        }
        core::set_config(s.config.clone());
    }
    Guard(guard)
}

impl State {
    fn set_config(&mut self, game: &Game<'_>, config: Config) {
        if config == self.config {
            return;
        }
        self.config = config;
        self.models.clear();
        self.arms.clear();
        core::set_config(self.config.clone());
        let dir = game.native_data_dir(ID);
        let saved = std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::write(dir.join("config.txt"), self.config.write()));
        if let Err(e) = saved {
            log::warn!("{ID}: could not save its settings: {e}");
        }
    }

    /// The third-person model for `skin`, built once per (skin, slim, flat hat) and config.
    fn model(
        &mut self,
        game: &Game<'_>,
        kind: &MobType,
        skin: &str,
        slim: bool,
        flat_hat: bool,
    ) -> Option<Arc<Part>> {
        let slot = usize::from(slim) * 2 + usize::from(flat_hat);
        if let Some(model) = self.models.get(skin).and_then(|m| m[slot].clone()) {
            return Some(model);
        }
        let downloads = game.skin_downloads.borrow();
        let image = downloads.image(skin)?;
        let key = if slim { "player_slim#main" } else { "main" };
        let base = kind.layer(key).unwrap_or(&kind.model);
        let built = Arc::new(build(base, image, slim, &self.config, Mode::ThirdPerson, flat_hat)?);
        if self.models.len() >= CACHE_LIMIT {
            self.models.clear();
        }
        self.models.entry(skin.to_owned()).or_default()[slot] = Some(built.clone());
        Some(built)
    }

    /// The first-person `arm` (`right_arm`/`left_arm` with its sleeve) for `skin`.
    fn arm(&mut self, game: &Game<'_>, arm: &Part, skin: &str, slim: bool, left: bool) -> Option<Arc<Part>> {
        let slot = usize::from(slim) * 2 + usize::from(left);
        if let Some(model) = self.arms.get(skin).and_then(|m| m[slot].clone()) {
            return Some(model);
        }
        let downloads = game.skin_downloads.borrow();
        let image = downloads.image(skin)?;
        let built = Arc::new(build(arm, image, slim, &self.config, Mode::FirstPerson, false)?);
        if self.arms.len() >= CACHE_LIMIT {
            self.arms.clear();
        }
        self.arms.entry(skin.to_owned()).or_default()[slot] = Some(built.clone());
        Some(built)
    }
}

/// `model` with each enabled outer-layer part's polygons replaced by its
/// voxels (the hat left flat when `flat_hat`), or `None` when the skin image
/// is not a full 64×64 RGBA image.
fn build(model: &Part, rgba: &[u8], slim: bool, config: &Config, mode: Mode, flat_hat: bool) -> Option<Part> {
    // A skin too small for any layer: nothing to build.
    core::layer_quads("hat", slim, rgba, config, mode)?;
    let mut out = model.clone();
    replace(&mut out, rgba, slim, config, mode, flat_hat);
    Some(out)
}

fn replace(part: &mut Part, rgba: &[u8], slim: bool, config: &Config, mode: Mode, flat_hat: bool) {
    let wanted = config.enabled(&part.name) && !(flat_hat && part.name == "hat");
    if wanted && let Some(quads) = core::layer_quads(&part.name, slim, rgba, config, mode) {
        part.polygons = quads
            .into_iter()
            .map(|q| Polygon {
                normal: q.normal,
                vertices: q.vertices,
                cull_back: false,
            })
            .collect();
    }
    for child in &mut part.children {
        replace(child, rgba, slim, config, mode, flat_hat);
    }
}

// ---------------------------------------------------------------------------
// What the hooks call
// ---------------------------------------------------------------------------

/// `skin_3d_model`: our model for `kind` wearing `skin`, `None` while the
/// image is not in memory yet (the frame hook tries again).
pub(super) fn model_for(game: &Game<'_>, kind: &Arc<MobType>, skin: &str, slim: bool) -> Option<Arc<Part>> {
    state(game).model(game, kind, skin, slim, false)
}

fn hides_hat(head: Option<&str>) -> bool {
    head.is_some_and(|item| HIDE_HAT.contains(&item))
}

/// Once a frame: every player within the level-of-detail distance wears the
/// 3D layer, every other one the vanilla flat layer (`PlayerModelMixin`'s
/// `renderDistanceLOD` check); the hat stays flat under a mob's head; and a
/// model the skin update could not build yet is tried again.
pub(super) fn frame(game: &mut Game<'_>) {
    let mut s = state(game);
    if s.sprites_age == 0 {
        // Which atlas sprite each skin is, for the head renderers (skulls and items).
        let sprites = &game.gui.atlas.sprites;
        let map = core::locations()
            .into_iter()
            .filter_map(|location| {
                let r = sprites.get(&location)?;
                Some((core::sprite_key([r.u0, r.v0, r.u1, r.v1]), location))
            })
            .collect();
        core::set_sprites(map);
    }
    s.sprites_age = (s.sprites_age + 1) % SPRITES_EVERY;
    let lod = f64::from(s.config.lod_distance);
    let lod2 = lod * lod;
    let viewer = game.player.as_ref().map(|p| [p.position.x, p.position.y, p.position.z]);
    // Wanted models first (reading the game), then assigned (writing it).
    let mut wanted: Vec<(usize, Option<Arc<Part>>)> = Vec::new();
    for (i, m) in game.mobs.iter().enumerate() {
        if &*m.kind.id != "minecraft:player" {
            continue;
        }
        let Some(skin) = m.skin.as_deref() else {
            continue;
        };
        let far = viewer.is_some_and(|v| {
            let (dx, dy, dz) = (m.position.x - v[0], m.position.y - v[1], m.position.z - v[2]);
            dx * dx + dy * dy + dz * dz > lod2
        });
        let want = if far {
            None
        } else {
            let flat_hat = hides_hat(m.extra.head.as_deref());
            s.model(game, &m.kind, skin, m.slim, flat_hat)
        };
        if !same(&m.skin_3d, &want) {
            wanted.push((i, want));
        }
    }
    let own_head = game.armor[0].as_ref().map(|stack| &*stack.item);
    let own = game.player_model.as_ref().and_then(|m| {
        let skin = m.skin.as_deref()?;
        let want = s.model(game, &m.kind, skin, m.slim, hides_hat(own_head));
        (!same(&m.skin_3d, &want)).then_some(want)
    });
    drop(s);
    for (i, want) in wanted {
        if let Some(m) = game.mobs.get_mut(i) {
            m.skin_3d = want;
        }
    }
    if let (Some(want), Some(m)) = (own, game.player_model.as_mut()) {
        m.skin_3d = want;
    }
}

fn same(a: &Option<Arc<Part>>, b: &Option<Arc<Part>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// `first_person_arm`: draws the arm with its 3D sleeve and breaks, or
/// continues to Horizon's own arm when the sleeve is off or the skin isn't
/// in memory.
pub(super) fn first_person_arm(
    game: &Game<'_>,
    left: bool,
    m: glam::Mat4,
    light: u32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow;
    let mut s = state(game);
    let enabled = if left {
        s.config.left_sleeve
    } else {
        s.config.right_sleeve
    };
    if !enabled {
        return ControlFlow::Continue(());
    }
    let Some(own) = game.player_model.as_ref() else {
        return ControlFlow::Continue(());
    };
    let slim = own.slim;
    let Some(skin) = own.skin.as_deref() else {
        return ControlFlow::Continue(());
    };
    let Some(sprite) = game.gui.atlas.sprites.get(skin).copied() else {
        return ControlFlow::Continue(());
    };
    let Some(base) = game.res.player_arm(slim, left).as_ref() else {
        return ControlFlow::Continue(());
    };
    let Some(arm) = s.arm(game, base, skin, slim, left) else {
        return ControlFlow::Continue(());
    };
    drop(s);
    // As Horizon's own `first_person_arm`: the rest pose turned 0.1 radians outwards.
    let z_rot = if left { -0.1 } else { 0.1 };
    let pose = |p: &Part| {
        let mut pose = p.pose;
        pose[3] = 0.0;
        pose[4] = 0.0;
        pose[5] = if p.name == arm.name { z_rot } else { 0.0 };
        pose
    };
    horizon_client::mobs::render_model(
        &arm,
        m,
        &pose,
        [sprite.u0, sprite.v0, sprite.u1, sprite.v1],
        light,
        0,
        out,
    );
    ControlFlow::Break(())
}

pub(super) fn list(mods: &mut Vec<crate::mod_api::NativeMod>) {
    mods.push(crate::mod_api::NativeMod {
        id: ID.to_owned(),
        title: format!("{TITLE} {VERSION}"),
        description: "Renders the player skin layer in 3D. A port of tr7zw's 3D Skin Layers \
                      (tr7zw Protective License)."
            .to_owned(),
        keys: Vec::new(),
    });
}

// ---------------------------------------------------------------------------
// The settings page: the original's three tabs, one after another
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Setting {
    Hat,
    Jacket,
    LeftSleeve,
    RightSleeve,
    LeftPants,
    RightPants,
    VoxelSize,
    HeadVoxelSize,
    TorsoVoxelWidth,
    FirstPersonVoxelSize,
    LodDistance,
    Skulls,
    SkullItems,
    SkullVoxelSize,
    FastRender,
    Reset,
}

const PAGE: [Setting; 16] = [
    // Players
    Setting::Hat,
    Setting::Jacket,
    Setting::LeftSleeve,
    Setting::RightSleeve,
    Setting::LeftPants,
    Setting::RightPants,
    Setting::VoxelSize,
    Setting::HeadVoxelSize,
    Setting::TorsoVoxelWidth,
    Setting::FirstPersonVoxelSize,
    Setting::LodDistance,
    // Player Heads
    Setting::Skulls,
    Setting::SkullItems,
    Setting::SkullVoxelSize,
    // Other Settings
    Setting::FastRender,
    Setting::Reset,
];

const VOXEL_SIZES: [f32; 9] = [1.0, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3, 1.4, 1.5];
const HEAD_SIZES: [f32; 9] = [1.0, 1.1, 1.15, 1.18, 1.2, 1.25, 1.3, 1.4, 1.5];
const TORSO_WIDTHS: [f32; 7] = [1.0, 1.02, 1.05, 1.08, 1.1, 1.15, 1.2];
const FIRST_PERSON_SIZES: [f32; 6] = [1.0, 1.05, 1.1, 1.15, 1.2, 1.3];
const SKULL_SIZES: [f32; 8] = [1.0, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3, 1.4];
const LOD_DISTANCES: [i32; 10] = [4, 8, 12, 14, 16, 24, 32, 48, 64, 128];

fn label(setting: Setting, c: &Config) -> String {
    let on = |b: bool| if b { "ON" } else { "OFF" };
    match setting {
        Setting::Hat => format!("3D Hat: {}", on(c.hat)),
        Setting::Jacket => format!("3D Jacket: {}", on(c.jacket)),
        Setting::LeftSleeve => format!("3D Left Sleeve: {}", on(c.left_sleeve)),
        Setting::RightSleeve => format!("3D Right Sleeve: {}", on(c.right_sleeve)),
        Setting::LeftPants => format!("3D Left Pants: {}", on(c.left_pants)),
        Setting::RightPants => format!("3D Right Pants: {}", on(c.right_pants)),
        Setting::VoxelSize => format!("Voxel Size: {:.2}", c.base_voxel_size),
        Setting::HeadVoxelSize => format!("Head Voxel Size: {:.2}", c.head_voxel_size),
        Setting::TorsoVoxelWidth => format!("Torso Voxel Width: {:.2}", c.body_voxel_width_size),
        Setting::FirstPersonVoxelSize => {
            format!("First Person Voxel Size: {:.2}", c.first_person_pixel_scaling)
        }
        Setting::LodDistance => format!("Level Of Detail Distance: {} blocks", c.lod_distance),
        Setting::Skulls => format!("3D Skulls: {}", on(c.skulls)),
        Setting::SkullItems => format!("3D Skull Items: {}", on(c.skull_items)),
        Setting::SkullVoxelSize => format!("Skull Voxel Size: {:.2}", c.skull_voxel_size),
        Setting::FastRender => format!("Fast Render: {}", on(c.fast_render)),
        Setting::Reset => "Reset to defaults".to_owned(),
    }
}

fn apply(setting: Setting, c: &mut Config) {
    match setting {
        Setting::Hat => c.hat = !c.hat,
        Setting::Jacket => c.jacket = !c.jacket,
        Setting::LeftSleeve => c.left_sleeve = !c.left_sleeve,
        Setting::RightSleeve => c.right_sleeve = !c.right_sleeve,
        Setting::LeftPants => c.left_pants = !c.left_pants,
        Setting::RightPants => c.right_pants = !c.right_pants,
        Setting::VoxelSize => c.base_voxel_size = cycle(c.base_voxel_size, &VOXEL_SIZES),
        Setting::HeadVoxelSize => c.head_voxel_size = cycle(c.head_voxel_size, &HEAD_SIZES),
        Setting::TorsoVoxelWidth => {
            c.body_voxel_width_size = cycle(c.body_voxel_width_size, &TORSO_WIDTHS)
        }
        Setting::FirstPersonVoxelSize => {
            c.first_person_pixel_scaling = cycle(c.first_person_pixel_scaling, &FIRST_PERSON_SIZES)
        }
        Setting::LodDistance => {
            c.lod_distance = LOD_DISTANCES
                .iter()
                .copied()
                .find(|&d| d > c.lod_distance)
                .unwrap_or(LOD_DISTANCES[0])
        }
        Setting::Skulls => c.skulls = !c.skulls,
        Setting::SkullItems => c.skull_items = !c.skull_items,
        Setting::SkullVoxelSize => c.skull_voxel_size = cycle(c.skull_voxel_size, &SKULL_SIZES),
        Setting::FastRender => c.fast_render = !c.fast_render,
        Setting::Reset => *c = Config::default(),
    }
}

/// A label starting with this is a half-width button, two to a row (the
/// `pair` hook on Options > Mods lays them out).
const HALF: char = '\u{1}';

/// The six part switches sit two to a row, as on the original's Players tab.
fn half_width(setting: Setting) -> bool {
    matches!(
        setting,
        Setting::Hat
            | Setting::Jacket
            | Setting::LeftSleeve
            | Setting::RightSleeve
            | Setting::LeftPants
            | Setting::RightPants
    )
}

pub(super) fn settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }
    let c = state(game).config.clone();
    buttons.extend(PAGE.iter().map(|&s| {
        let text = label(s, &c);
        if half_width(s) { format!("{HALF}{text}") } else { text }
    }));
}

pub(super) fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    let Some(&setting) = PAGE.get(button) else {
        return;
    };
    let mut s = state(game);
    let mut c = s.config.clone();
    apply(setting, &mut c);
    s.set_config(game, c);
}

/// The first preset above `value`, wrapping round to the smallest.
fn cycle(value: f32, presets: &[f32]) -> f32 {
    presets
        .iter()
        .copied()
        .find(|&p| p > value + 1e-4)
        .unwrap_or(presets[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_has_a_label_and_a_click() {
        let mut c = Config::default();
        for setting in PAGE {
            assert!(!label(setting, &c).is_empty());
            apply(setting, &mut c);
        }
        // The last button resets.
        assert_eq!(c, Config::default());
    }

    #[test]
    fn the_part_switches_come_in_pairs() {
        // Six half-width buttons first, so they make three full rows of two.
        let halves: Vec<bool> = PAGE.iter().map(|&s| half_width(s)).collect();
        assert_eq!(halves.iter().take_while(|&&h| h).count(), 6);
        assert_eq!(halves.iter().filter(|&&h| h).count(), 6);
    }

    #[test]
    fn mob_heads_flatten_the_hat_but_player_heads_do_not() {
        assert!(hides_hat(Some("minecraft:creeper_head")));
        assert!(!hides_hat(Some("minecraft:player_head")));
        assert!(!hides_hat(Some("minecraft:diamond_helmet")));
        assert!(!hides_hat(None));
    }

    #[test]
    fn cycling_wraps_round() {
        assert_eq!(cycle(1.5, &VOXEL_SIZES), 1.0);
        assert_eq!(cycle(1.15, &VOXEL_SIZES), 1.2);
        assert_eq!(cycle(1.17, &VOXEL_SIZES), 1.2, "a hand-edited value moves to the next preset");
    }
}
