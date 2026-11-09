//! The custom textures a pipeline's programs read: an image of the pack (a PNG or a raw dump,
//! decoded) or a resource location the game resolves, bound to a sampler name in the programs of
//! a stage (`horizon-shaders` reads them from `shaders.properties`, spec 3.4).

use crate::Format;

/// The stage of a `customTexture.<name>`: every program's.
pub const EVERY_STAGE: &str = "*";

/// A texture bound to sampler `name` in the programs of `stage` (`gbuffers`, `deferred`,
/// `composite`...; [`EVERY_STAGE`] for a `customTexture`), or the noise texture (stage and
/// name `noise`).
#[derive(Clone, Debug)]
pub struct CustomTexture {
    pub stage: String,
    pub name: String,
    pub source: TextureSource,
}

#[derive(Clone, Debug)]
pub enum TextureSource {
    Image(Texture),
    /// A resource location for the game to resolve.
    Resource(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dim {
    D1,
    D2,
    D3,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub dim: Dim,
    /// Width, height and depth (1 for the dimensions the texture lacks).
    pub size: [u32; 3],
    /// The internal format the pack asked for.
    pub format: Format,
    pub texels: Texels,
    /// Linear filtering, and clamping instead of repeating: on for a raw texture and off for a
    /// PNG, unless the `.mcmeta` beside it says otherwise.
    pub blur: bool,
    pub clamp: bool,
    /// A raw dump rather than a PNG.
    pub raw: bool,
}

/// Texel values, `components` per texel: 1, 2 or 4 (three widen to four, alpha 1).
#[derive(Clone, Debug, PartialEq)]
pub enum Texels {
    Float(Vec<f32>),
    Int(Vec<i32>),
    Uint(Vec<u32>),
}

impl Texture {
    pub fn components(&self) -> usize {
        components(self.format.channels)
    }
}

/// The components per texel a texture of `channels` channels stores: three widen to four.
pub fn components(channels: u8) -> usize {
    match channels {
        1 => 1,
        2 => 2,
        _ => 4,
    }
}
