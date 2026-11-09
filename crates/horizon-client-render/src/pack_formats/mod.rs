//! The resource pack formats vanilla doesn't read (OptiFine's, Polytone's) on the engine's
//! extension points ([`crate::pack_hooks`]). This is platform code (docs/layers.md) kept in this
//! crate until the platform's executable installs it: it uses only the crate's public API, so it
//! moves to a platform crate unchanged, and nothing else in the crate names it.

pub(crate) mod block_entity_cem;
pub mod cit;
mod cit_glint;
mod colors;
mod connected;
mod ctm;
mod custom_sky;
mod entity_textures;
pub mod environment;
mod fusion;
mod pack_options;
mod post;

use std::path::Path;
use std::sync::Arc;

use crate::pack_hooks::PackFormats;

/// OptiFine's and Polytone's formats.
pub struct Formats;

impl PackFormats for Formats {
    fn colors(&self, root: &Path) -> Option<crate::pack_palette::PackColors> {
        Some(colors::read(root))
    }

    fn connected_textures(
        &self,
        root: &Path,
        registry: &horizon_world::registry::BlockRegistry,
        sprites: &mut crate::sprites::Sprites,
        models: &mut crate::models::StateModels,
        native_mods: &mut dyn FnMut(&mut crate::sprites::Sprites),
    ) -> Option<crate::connected::Connected> {
        connected::load(root, registry, sprites, models, native_mods)
    }

    fn emissive_suffix(&self, root: &Path) -> Option<String> {
        Some(horizon_optifine::emissive::suffix(root))
    }

    fn item_textures(&self, root: &Path) -> Option<crate::cit::CitRules> {
        Some(cit::load(root))
    }

    fn entity_textures(&self, root: &Path) -> Option<crate::entity_textures::EntityTextures> {
        Some(entity_textures::load(root))
    }

    fn block_entity_models(
        &self,
        root: &Path,
        vanilla: &std::collections::HashMap<&'static str, crate::block_entity_render::ModelPart>,
    ) -> Option<crate::block_entity_render::Reshaped> {
        block_entity_cem::reshape(root, vanilla)
    }

    fn custom_sky(&self, root: &Path) -> Option<Arc<dyn crate::custom_sky::SkyLayers>> {
        let sky = custom_sky::CustomSky::load(root);
        (!sky.is_empty()).then(|| Arc::new(sky) as _)
    }

    fn environment_modifiers(
        &self,
        root: &Path,
    ) -> Option<Box<dyn crate::environment::EnvironmentModifiers>> {
        Some(Box::new(environment::Modifiers::read(root)))
    }

    fn colormap_targets(&self, root: &Path) -> Vec<crate::environment::ColormapTarget> {
        environment::colormap_targets(root)
    }

    fn block_colormap(
        &self,
        root: &Path,
        id: &str,
    ) -> Option<Arc<dyn crate::environment::BlockColormap>> {
        Some(Arc::new(environment::Colormap::load(root, id)?))
    }

    fn post_chains(
        &self,
        root: &Path,
        options: horizon_pack_options::Configs,
    ) -> Option<Box<dyn crate::polytone_post::PostChains>> {
        Some(Box::new(post::Post::read(root, options)?))
    }

    fn frame_expression(
        &self,
        source: &str,
    ) -> Option<Box<dyn crate::environment::FrameExpression>> {
        match horizon_polytone::expr::Program::compile(source) {
            Ok(program) => Some(Box::new(environment::FrameProgram(program))),
            Err(e) => {
                log::warn!("expression {source:?}: {e}");
                None
            }
        }
    }

    fn pack_options(&self, source: &crate::packs::Source) -> Option<horizon_pack_options::Configs> {
        Some(pack_options::read(source))
    }

    fn overlay_active(
        &self,
        entry: &serde_json::Value,
        options: &horizon_pack_options::Configs,
        formats_apply: bool,
    ) -> bool {
        horizon_polytone::pack::overlay_active(entry, options, formats_apply)
    }

    fn sounds_without_unmet(
        &self,
        text: &[u8],
        options: &horizon_pack_options::Configs,
    ) -> Option<Vec<u8>> {
        horizon_polytone::pack::sounds_without_unmet(text, options)
    }
}

/// Installs these formats ([`crate::pack_hooks::install`]); the game does at startup.
pub fn install() {
    crate::pack_hooks::install(&Formats);
}
