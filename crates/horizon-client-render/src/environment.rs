//! A dimension's environment attributes (`EnvironmentAttributeSystem.addDefaultLayers`): the
//! dimension type's attributes, the biome layer, the dimension's timelines and, where it can
//! have weather, the rain and thunder layers, sampled into the sky state the renderer draws.
use crate::Result;
use horizon_world::environment_attributes::{AttributeMap, Value, attribute};
use horizon_world::environment_system::{BiomeBlend, Builder, Conditions, System};
use horizon_world::environment_timeline::Timeline;
use serde_json::Value as Json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub struct Environment {
    system: System,
    /// What the packs' expressions read; the frame sets it before sampling.
    frame: Arc<RwLock<Frame>>,
    /// Biome attribute maps by biome id, where resource packs changed them; the biome layer
    /// reads these instead of the biome's own.
    biomes: HashMap<String, AttributeMap>,
    /// Packs' biome `post_process` maps by biome id: a biome layer after the weather.
    post: HashMap<String, AttributeMap>,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Sky {
    pub sky_color: [f32; 3],
    pub fog_color: [f32; 3],
    pub sky_light_factor: f32,
    pub sky_light_color: [f32; 3],
    pub block_light_tint: [f32; 3],
    pub ambient_light_color: [f32; 3],
    pub sun_angle: f32,
    pub moon_angle: f32,
    pub star_angle: f32,
    pub star_brightness: f32,
    pub sunrise_color: [f32; 4],
    pub cloud_color: [f32; 4],
    /// `gameplay/sky_light_level`, the server-side sky darkening.
    pub sky_light_level: f32,
    /// Environmental fog (`visual/fog_start_distance` / `fog_end_distance`).
    pub fog_start: f32,
    pub fog_end: f32,
    /// `visual/cloud_fog_end_distance`.
    pub cloud_fog_end: f32,
    /// `visual/cloud_height`.
    pub cloud_height: f32,
}

/// Where and when the sky is sampled.
#[derive(Clone, Copy, Default)]
pub struct Sample<'a> {
    /// The day clock's total ticks.
    pub day_time: i64,
    /// `Level.getRainLevel` and `getThunderLevel`, 0 where the dimension has no weather.
    pub rain: f32,
    pub thunder: f32,
    /// The biomes around the camera (`EnvironmentAttributeProbe`'s Gaussian mix).
    pub blend: Option<&'a BiomeBlend<'a>>,
    /// The biome at the camera, for attributes that don't blend.
    pub biome: Option<&'a AttributeMap>,
    /// The same for the packs' biome post-process group.
    pub post_blend: Option<&'a BiomeBlend<'a>>,
    pub post_biome: Option<&'a AttributeMap>,
}

/// What resource packs change in the attribute stacks (Polytone's biome and dimension
/// modifiers, #11486), read by the platform's pack formats once per pack load. Without
/// installed formats, or packs with no modifiers, it changes nothing.
#[derive(Default)]
pub struct PackModifiers(Option<Box<dyn EnvironmentModifiers>>);

impl PackModifiers {
    /// The modifiers of the packs merged at the overlay root `pack_root`.
    pub fn read(pack_root: &Path) -> Self {
        Self(crate::pack_hooks::formats().environment_modifiers(pack_root))
    }

    pub fn is_empty(&self) -> bool {
        self.0.as_ref().is_none_or(|m| m.is_empty())
    }

    /// The packs' option values, which the modifiers' expressions read.
    pub fn with_configs(mut self, configs: horizon_pack_options::Configs) -> Self {
        if let Some(m) = &mut self.0 {
            m.set_options(configs);
        }
        self
    }
}

/// A pack format's changes to the attribute stacks: entries set, replaced (maybe by a
/// [`horizon_world::environment_attributes::Dynamic`] reading [`Frame`]) or removed.
pub trait EnvironmentModifiers: Send + Sync {
    fn is_empty(&self) -> bool;

    /// The packs' option values.
    fn set_options(&mut self, options: horizon_pack_options::Configs);

    /// Changes the maps of dimension type `id` (`in_tag`: whether it is in a
    /// `dimension_type` tag). Dynamic entries read `frame`.
    fn dimension(
        &self,
        id: &str,
        in_tag: &dyn Fn(&str) -> bool,
        frame: &Arc<RwLock<Frame>>,
        maps: DimensionMaps<'_>,
    );

    /// Changes biome `id`'s map (`in_tag`: whether it is in a `worldgen/biome` tag), and fills
    /// its post-process map, a biome layer after the weather.
    fn biome(
        &self,
        id: &str,
        in_tag: &dyn Fn(&str) -> bool,
        frame: &Arc<RwLock<Frame>>,
        base: &mut AttributeMap,
        post: &mut AttributeMap,
    );
}

/// A dimension's maps the modifiers change: its own entries, the weather's, and a layer
/// after all others.
pub struct DimensionMaps<'a> {
    pub base: &'a mut AttributeMap,
    pub rain: &'a mut AttributeMap,
    pub thunder: &'a mut AttributeMap,
    pub post: &'a mut AttributeMap,
}

/// A pack's colormap over blocks (Polytone's block and fluid colormaps, wiki "Colormaps").
pub trait BlockColormap: Send + Sync {
    /// Whether blocks take the mean of the colours around them, over the Biome Blend square.
    fn biome_blend(&self) -> bool;

    /// The colour at block `at`, in `biome`, ARGB; `None` where the pixel is transparent or
    /// an axis fails (the block keeps its own tint).
    fn sample(&self, at: [i32; 3], biome: &crate::biome::BiomeInfo) -> Option<u32>;
}

/// A block or fluid modifier with a colormap: the blocks it targets (by name) and the
/// colormap's id.
pub struct ColormapTarget {
    pub fluid: bool,
    pub targets: Box<dyn Fn(&str) -> bool + Send + Sync>,
    pub colormap: String,
}

/// What a pack's expression can ask about the frame (wiki "Scripting Expressions": the global,
/// player, camera and positional contexts), set before the frame samples the sky.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub rain: f32,
    pub thunder: f32,
    pub game_time: i64,
    pub day_time: i64,
    /// The render distance in chunks.
    pub render_distance: i32,
    /// `skyType`: 1 for the overworld's sky, 2 for the End's, 0 for none.
    pub sky_type: i32,
    pub dimension: String,
    pub player: [f64; 3],
    /// The player's motion per tick (`p.speedSq`).
    pub player_velocity: [f64; 3],
    pub player_on_ground: bool,
    pub in_water: bool,
    pub camera: [f64; 3],
    /// The sky and block light at the camera.
    pub sky_light: i32,
    pub block_light: i32,
    /// The camera's biome (`o.biome()`), and its climate (`o.temperature`, `o.downfall`).
    pub biome: String,
    pub temperature: f32,
    pub downfall: f32,
}

impl Frame {
    /// `g.rain`: the mean of the rain and thunder levels, so plain rain is 0.5 and a
    /// thunderstorm 1 (observed on the real mod: a pack's `g.rain` colormap row).
    pub fn global_rain(&self) -> f64 {
        (f64::from(self.rain) + f64::from(self.thunder)) / 2.0
    }
}

/// The timeline ids a dimension type's `timelines` names: an id, a `#tag` or a list,
/// tags resolved through `tags/timeline/` (their entries may be `{"id", "required"}`).
fn timeline_ids(root: &Path, json: &Json, out: &mut Vec<String>, depth: u32) {
    let entry = |e: &Json| {
        e.as_str()
            .or_else(|| e.get("id").and_then(Json::as_str))
            .map(str::to_owned)
    };
    let items: Vec<String> = match json {
        Json::Array(list) => list.iter().filter_map(entry).collect(),
        other => entry(other).into_iter().collect(),
    };
    for item in items {
        if let Some(tag) = item.strip_prefix('#') {
            let (namespace, path) = tag.split_once(':').unwrap_or(("minecraft", tag));
            let file = root.join(format!("data/{namespace}/tags/timeline/{path}.json"));
            if depth < 8
                && let Ok(tag) = horizon_world::json_file::read::<Json>(&file)
            {
                timeline_ids(root, &tag["values"], out, depth + 1);
            }
        } else if !out.contains(&item) {
            out.push(item);
        }
    }
}

impl Environment {
    pub fn load(root: &Path) -> Result<Self> {
        Self::load_dimension(root, "overworld")
    }

    /// `EnvironmentAttributeSystem.addDefaultLayers` for a dimension type: its attributes, a
    /// biome layer for every attribute some biome sets, its timelines (in tag order), and the
    /// weather where `Level.canHaveWeather` (sky light, no ceiling, not the End).
    pub fn load_dimension(root: &Path, dimension: &str) -> Result<Self> {
        Self::load_dimension_with(root, dimension, &PackModifiers::default())
    }

    /// The same with resource packs' Polytone modifiers (wiki "Environment Attributes"): a
    /// dimension modifier's `base` changes the dimension's own entries, `rain` and `thunder`
    /// the weather's, `post_process` adds a layer after all others; a biome modifier's `base`
    /// changes the biome's entries, `post_process` adds a biome layer after the weather.
    /// Modifiers apply lowest priority first, so the highest wins.
    pub fn load_dimension_with(
        root: &Path,
        dimension: &str,
        packs: &PackModifiers,
    ) -> Result<Self> {
        let json: Json = horizon_world::json_file::read(
            &root.join(format!("data/minecraft/dimension_type/{dimension}.json")),
        )?;
        let id = format!("minecraft:{dimension}");
        let tags = crate::pack_tags::Tags::new(root);
        let frame = Arc::new(RwLock::new(Frame::default()));
        let mut dimension_map = AttributeMap::parse(&json["attributes"])?;
        let (mut rain, mut thunder) = horizon_world::environment_system::weather_maps();
        let mut dimension_post = AttributeMap::default();
        if let Some(m) = &packs.0 {
            let in_tag = |tag: &str| tags.contains("dimension_type", tag, &id);
            let maps = DimensionMaps {
                base: &mut dimension_map,
                rain: &mut rain,
                thunder: &mut thunder,
                post: &mut dimension_post,
            };
            m.dimension(&id, &in_tag, &frame, maps);
        }
        let mut builder = Builder::default();
        builder.constant_layer(&dimension_map);
        // Without biome data (a bare dimension type), there is no biome layer.
        let mut biomes = HashMap::new();
        let mut post = HashMap::new();
        if let Ok(folder) =
            horizon_world::json_folder::read(&root.join("data/minecraft/worldgen/biome"))
        {
            for file in folder.files.iter().filter(|f| !f.name.contains('/')) {
                let biome: Json = horizon_world::json_file::parse(&file.path, file.bytes()?)?;
                let name = format!("minecraft:{}", file.name);
                let mut map = match AttributeMap::parse(&biome["attributes"]) {
                    Ok(map) => map,
                    Err(e) => {
                        log::warn!("biome {}: {e}", file.name);
                        continue;
                    }
                };
                let mut post_map = AttributeMap::default();
                if let Some(m) = &packs.0 {
                    let in_tag = |tag: &str| tags.contains("worldgen/biome", tag, &name);
                    m.biome(&name, &in_tag, &frame, &mut map, &mut post_map);
                }
                if !post_map.is_empty() {
                    post.insert(name.clone(), post_map);
                }
                biomes.insert(name, map);
            }
        }
        builder.biome_layer(biomes.values());
        let mut ids = Vec::new();
        timeline_ids(root, &json["timelines"], &mut ids, 0);
        for id in ids {
            let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", &id));
            let file = root.join(format!("data/{namespace}/timeline/{path}.json"));
            match horizon_world::json_file::read::<Json>(&file)
                .map_err(|e| e.to_string())
                .and_then(|t| Timeline::parse(&t))
            {
                Ok(timeline) => {
                    builder.timeline_layer(&timeline);
                }
                Err(e) => log::warn!("timeline {id}: {e}"),
            }
        }
        let weather = json["has_skylight"].as_bool() == Some(true)
            && json["has_ceiling"].as_bool() != Some(true)
            && dimension != "the_end";
        if weather {
            builder.weather_layers_with(&rain, &thunder);
        }
        builder.biome_post_layer(post.values());
        builder.constant_layer(&dimension_post);
        Ok(Self {
            system: builder.build(),
            frame,
            biomes,
            post,
        })
    }

    /// Sets what the packs' expressions read for the next samples.
    pub fn set_frame(&self, frame: Frame) {
        if let Ok(mut f) = self.frame.write() {
            *f = frame;
        }
    }

    /// A biome's attribute map (by id, `minecraft:plains`), as the packs left it.
    pub fn biome(&self, id: &str) -> Option<&AttributeMap> {
        self.biomes.get(id)
    }

    /// A biome's post-process map from the packs, if any sets one.
    pub fn biome_post(&self, id: &str) -> Option<&AttributeMap> {
        self.post.get(id)
    }

    /// Whether packs added post-process biome layers (the frame then mixes them too).
    pub fn has_biome_post(&self) -> bool {
        !self.post.is_empty()
    }

    /// `EnvironmentAttributeSystem.getValue` of one attribute, or its dimension value without
    /// a biome.
    pub fn value(&self, id: &str, at: &Sample) -> Option<Value> {
        let attribute = attribute(id)?;
        let biome = |_full_resolution: bool| at.biome;
        let clock = |_clock: &str| at.day_time;
        let post_biome = |_full_resolution: bool| at.post_biome;
        let conditions = Conditions {
            rain: at.rain,
            thunder: at.thunder,
            blend: at.blend,
            biome: &biome,
            clock: &clock,
            post_blend: at.post_blend,
            post_biome: Some(&post_biome),
        };
        match (at.blend, at.biome) {
            (None, None) => self.system.dimension_value(attribute, &conditions),
            _ => self.system.value(attribute, &conditions),
        }
    }

    /// The sky state the renderer draws.
    pub fn sample(&self, at: &Sample) -> Sky {
        let float = |id: &str| match self.value(id, at) {
            Some(Value::Float(f)) => f,
            _ => 0.0,
        };
        let rgb = |id: &str| match self.value(id, at) {
            Some(Value::Rgb(c)) => c,
            _ => [0.0; 3],
        };
        let argb = |id: &str| match self.value(id, at) {
            Some(Value::Argb(c)) => c,
            _ => [0.0; 4],
        };
        Sky {
            sky_color: rgb("visual/sky_color"),
            fog_color: rgb("visual/fog_color"),
            sky_light_factor: float("visual/sky_light_factor"),
            sky_light_color: rgb("visual/sky_light_color"),
            block_light_tint: rgb("visual/block_light_tint"),
            ambient_light_color: rgb("visual/ambient_light_color"),
            sun_angle: float("visual/sun_angle"),
            moon_angle: float("visual/moon_angle"),
            star_angle: float("visual/star_angle"),
            star_brightness: float("visual/star_brightness"),
            sunrise_color: argb("visual/sunrise_sunset_color"),
            cloud_color: argb("visual/cloud_color"),
            sky_light_level: float("gameplay/sky_light_level") / 15.0,
            fog_start: float("visual/fog_start_distance"),
            fog_end: float("visual/fog_end_distance"),
            cloud_fog_end: float("visual/cloud_fog_end_distance"),
            cloud_height: float("visual/cloud_height"),
        }
    }
}

/// `Level.updateSkyBrightness`'s darkening: from the sky light level, a dimension value.
pub fn sky_darken(environment: &Environment, day_time: i64, rain: f32, thunder: f32) -> i32 {
    let sky = environment.sample(&Sample {
        day_time,
        rain,
        thunder,
        ..Sample::default()
    });
    // Level.updateSkyBrightness: `(int)(15.0F - skyLightLevel)`, the float
    // truncated, not rounded (#11323): the darkening steps up only once the
    // light has fallen a whole level. `sky_light_level` is the attribute as a
    // fraction of 15, so the product is its raw value in `f32`.
    (15.0 - 15.0 * sky.sky_light_level) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overworld() -> Option<Environment> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/horizon-client/26.3");
        root.join("data/minecraft/timeline/day.json")
            .exists()
            .then(|| Environment::load(&root).unwrap())
    }

    #[test]
    fn dark_outside_follows_time_and_weather_levels() {
        let Some(env) = overworld() else {
            return;
        };
        let dark = |time, rain, thunder| sky_darken(&env, time, rain, thunder) >= 4;
        assert!(!dark(6000, 0.0, 0.0), "clear noon");
        assert!(!dark(6000, 1.0, 0.0), "rainy noon");
        assert!(dark(6000, 1.0, 1.0), "thunderstorm at noon");
        assert!(dark(18000, 0.0, 0.0), "clear midnight");
        assert!(!dark(1000, 0.0, 0.0), "clear morning");
    }

    /// A world with one sky-lit dimension and two biomes, one of them in a tag, and a pack with
    /// Polytone modifiers.
    fn with_packs(pack: &[(&str, &str)]) -> Environment {
        let files: Vec<(&str, &[u8])> = pack.iter().map(|(p, t)| (*p, t.as_bytes())).collect();
        with_pack_files(&files)
    }

    fn with_pack_files(pack: &[(&str, &[u8])]) -> Environment {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "horizon-env-packs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let write = |path: &str, text: &[u8]| {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(
            "world/data/minecraft/dimension_type/overworld.json",
            r##"{"has_skylight": true, "has_ceiling": false,
                "attributes": {"minecraft:visual/sky_color": "#808080", "minecraft:visual/fog_color": "#000000"}}"##.as_bytes(),
        );
        write(
            "world/data/minecraft/worldgen/biome/plains.json",
            r##"{"attributes": {"minecraft:visual/sky_color": "#404040"}}"##.as_bytes(),
        );
        write(
            "world/data/minecraft/worldgen/biome/desert.json",
            r#"{"attributes": {}}"#.as_bytes(),
        );
        write(
            "world/data/minecraft/tags/worldgen/biome/dry.json",
            r#"{"values": ["minecraft:desert"]}"#.as_bytes(),
        );
        for (path, text) in pack {
            write(&format!("pack/{path}"), text);
        }
        crate::pack_formats::install();
        let modifiers = PackModifiers::read(&dir.join("pack")).with_configs(
            crate::packs::polytone_configs(&crate::packs::Source::Dir(dir.join("pack"))),
        );
        let env =
            Environment::load_dimension_with(&dir.join("world"), "overworld", &modifiers).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        env
    }

    fn sky_at(env: &Environment, biome: &str, rain: f32) -> Sky {
        let map = env.biome(biome).unwrap();
        env.sample(&Sample {
            rain,
            biome: Some(map),
            post_biome: env.biome_post(biome),
            ..Sample::default()
        })
    }

    const GRAY_40: f32 = 64.0 / 255.0;

    #[test]
    fn pack_biome_modifiers_change_their_targets_only() {
        let env = with_packs(&[(
            "assets/mypack/polytone/biome_modifiers/dry_fog.json",
            r##"{"targets": "#minecraft:dry", "attributes_modifiers": {"visual/fog_color": "ff0000"}}"##,
        )]);
        assert_eq!(
            sky_at(&env, "minecraft:desert", 0.0).fog_color,
            [1.0, 0.0, 0.0]
        );
        assert_eq!(sky_at(&env, "minecraft:plains", 0.0).fog_color, [0.0; 3]);
        // Without packs the stack is vanilla's: the biome's own sky.
        let plain = with_packs(&[]);
        assert_eq!(
            sky_at(&plain, "minecraft:plains", 0.0).sky_color,
            [GRAY_40; 3]
        );
    }

    #[test]
    fn dimension_modifiers_remove_weather_layers_and_post_process_last() {
        let raining = |env: &Environment| sky_at(env, "minecraft:plains", 1.0).sky_color;
        // Vanilla's rain grays the sky toward a darker gray.
        assert_ne!(raining(&with_packs(&[])), [GRAY_40; 3]);
        let no_rain = with_packs(&[(
            "assets/minecraft/polytone/dimension_modifiers/overworld.json",
            r#"{"attributes_modifiers": {"rain": {"visual/sky_color": "REMOVE"}}}"#,
        )]);
        assert_eq!(raining(&no_rain), [GRAY_40; 3]);
        let post = with_packs(&[(
            "assets/minecraft/polytone/dimension_modifiers/overworld.json",
            r##"{"attributes_modifiers": {"post_process": {"visual/sky_color": "#0000ff"}}}"##,
        )]);
        assert_eq!(raining(&post), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn biome_post_process_and_priority() {
        let env = with_packs(&[
            (
                "assets/a/polytone/biome_modifiers/wins.json",
                r##"{"targets": "minecraft:plains", "priority": 5,
                    "attributes_modifiers": {"base": {"visual/fog_color": "#00ff00"},
                                             "post_process": {"visual/sky_color": "#ffffff"}}}"##,
            ),
            (
                "assets/b/polytone/biome_modifiers/loses.json",
                r##"{"targets": "minecraft:plains", "priority": -1,
                    "attributes_modifiers": {"visual/fog_color": "#0000ff"}}"##,
            ),
        ]);
        // The higher priority's fog wins.
        assert_eq!(
            sky_at(&env, "minecraft:plains", 0.0).fog_color,
            [0.0, 1.0, 0.0]
        );
        // The post-process sky comes after the weather, so the rain doesn't gray it.
        assert_eq!(
            sky_at(&env, "minecraft:plains", 1.0).sky_color,
            [1.0, 1.0, 1.0]
        );
        assert!(env.biome_post("minecraft:desert").is_none());
    }

    /// A number an expression gives outside the attribute's range is clamped into it, as vanilla
    /// clamps a sampled value: a fog end that a pack takes below zero deep underground is fog
    /// at once, not the default end.
    #[test]
    fn an_expression_value_out_of_range_is_clamped() {
        let env = with_packs(&[(
            "assets/minecraft/polytone/dimension_modifiers/overworld.json",
            r#"{"attributes_modifiers": {"visual/fog_end_distance": "64 - (p.y < 0) * 80"}}"#,
        )]);
        let end_at = |y: f64| {
            env.set_frame(Frame {
                player: [0.5, y, 0.5],
                ..Frame::default()
            });
            sky_at(&env, "minecraft:plains", 0.0).fog_end
        };
        assert_eq!(end_at(70.0), 64.0);
        assert_eq!(end_at(-2.0), 0.0);
    }

    #[test]
    fn expression_values_read_the_frame_and_the_value_so_far() {
        let env = with_packs(&[
            (
                "assets/minecraft/polytone/dimension_modifiers/overworld.json",
                r#"{"attributes_modifiers": {
                    "visual/fog_end_distance": "14 * (c.viewDistance / 16) + min(g.rain * 2, 1) * 100",
                    "visual/fog_start_distance": {"argument": "v + config('mypack:shift')", "modifier": "override"},
                    "visual/cloud_height": "v * 0.5"}}"#,
            ),
            (
                "assets/mypack/polytone/config_entries/shift.json",
                r#"{"default_value": 7}"#,
            ),
        ]);
        let at = |rain: f32, thunder: f32, chunks: i32| {
            env.set_frame(Frame {
                rain,
                thunder,
                render_distance: chunks,
                ..Frame::default()
            });
            sky_at(&env, "minecraft:plains", 0.0)
        };
        // `c.viewDistance` is in blocks: 16 chunks are 256.
        assert_eq!(at(0.0, 0.0, 16).fog_end, 224.0);
        // `g.rain` is the mean of rain and thunder: half rain is 0.25 (448 + 50), plain rain
        // 0.5 and a thunderstorm 1, both past the `min` (224 + 100).
        assert_eq!(at(0.5, 0.0, 32).fog_end, 498.0);
        assert_eq!(at(1.0, 0.0, 16).fog_end, 324.0);
        assert_eq!(at(1.0, 1.0, 16).fog_end, 324.0);
        // `v` is the value the layers before gave: the default start 0, and 192.33 for clouds.
        assert_eq!(at(0.0, 0.0, 16).fog_start, 7.0);
        assert_eq!(at(0.0, 0.0, 16).cloud_height, 192.33 * 0.5);
    }

    /// A PNG of RGBA pixels in one row.
    fn png_row(pixels: &[[u8; 4]]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, pixels.len() as u32, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&pixels.concat()).unwrap();
        writer.finish().unwrap();
        out
    }

    #[test]
    fn colormap_values_read_the_pixel_their_axes_pick() {
        // Left to right: red, green, transparent, blue; x 1 is the left edge.
        let png = png_row(&[
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [9, 9, 9, 0],
            [0, 0, 255, 255],
        ]);
        let env = with_pack_files(&[
            (
                "assets/minecraft/polytone/dimension_modifiers/overworld.json",
                br#"{"attributes_modifiers": {"visual/sky_color": "mypack:sky"}}"#,
            ),
            (
                "assets/mypack/polytone/colormaps/sky.json",
                br#"{"x_axis": "1 - g.rain", "y_axis": 0}"#,
            ),
            ("assets/mypack/polytone/colormaps/sky.png", &png),
        ]);
        let sky = |rain: f32, thunder: f32| {
            env.set_frame(Frame {
                rain,
                thunder,
                ..Frame::default()
            });
            env.sample(&Sample::default()).sky_color
        };
        // `g.rain` 0, 0.3 (rain 0.6), 1 (a thunderstorm).
        assert_eq!(sky(0.0, 0.0), [1.0, 0.0, 0.0]);
        assert_eq!(sky(0.6, 0.0), [0.0, 1.0, 0.0]);
        assert_eq!(sky(1.0, 1.0), [0.0, 0.0, 1.0]);
        // A transparent pixel keeps the colour the layers before gave (the dimension's #808080):
        // `g.rain` 0.7.
        assert_eq!(sky(1.0, 0.4), [128.0 / 255.0; 3]);
    }

    #[test]
    fn biome_id_colormaps_read_the_column_their_mapper_gives_the_biome() {
        // Left to right: red, green, transparent, blue.
        let png = png_row(&[
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [9, 9, 9, 0],
            [0, 0, 255, 255],
        ]);
        let env = with_pack_files(&[
            (
                "assets/minecraft/polytone/dimension_modifiers/overworld.json",
                br#"{"attributes_modifiers": {"visual/sky_color": "mypack:sky"}}"#,
            ),
            (
                "assets/mypack/polytone/colormaps/sky.json",
                br#"{"x_axis": "biome_id", "y_axis": 0, "biome_id_mapper": "mypack:biomes"}"#,
            ),
            ("assets/mypack/polytone/colormaps/sky.png", &png),
            (
                // Pixel columns from the left, over a texture_size of the image's width.
                "assets/mypack/polytone/biome_id_mappers/biomes.json",
                br#"{"minecraft:desert": 3, "swamp": 1, "minecraft:plains": 2, "texture_size": 4}"#,
            ),
        ]);
        let sky = |biome: &str| {
            env.set_frame(Frame {
                biome: biome.to_owned(),
                ..Frame::default()
            });
            env.sample(&Sample::default()).sky_color
        };
        assert_eq!(sky("minecraft:desert"), [0.0, 0.0, 1.0]);
        assert_eq!(sky("minecraft:swamp"), [0.0, 1.0, 0.0]);
        // A biome the mapper doesn't name is 0: the first column.
        assert_eq!(sky("minecraft:forest"), [1.0, 0.0, 0.0]);
        // Plains land on the transparent pixel: the colour before stays.
        assert_eq!(sky("minecraft:plains"), [128.0 / 255.0; 3]);
    }
}
