//! The external shader pipeline the renderer runs (docs/layers.md, layer plan M6): what a shader
//! pack hands the renderer once its own format is read. The renderer (`horizon-client-render`,
//! engine) builds its targets and passes from these types and does nothing with them on
//! vanilla's path; the pack formats that fill them are platform crates (`horizon-shaders` reads
//! `shaders/` packs). No wgpu and no pack file here.
//!
//! - [`config`]: the colortex buffers and the shadow map: formats, clearing, mipmaps, settings.
//! - [`stages`], [`inputs`], [`Api`]: the WGSL stage interface Horizon's own packs are written
//!   against (`horizon-shader-api` reads and checks those packs): the stages' functions, preludes,
//!   bindings and entry points, and the frame uniform the renderer fills.

mod api;
pub mod config;
pub mod inputs;
pub mod stages;

pub use api::Api;
pub use config::{
    BUFFERS, Buffer, Config, Format, HALF_LIVES, Kind, SHADOW_COLORS, Shadow, buffer_index,
    shadow_color,
};
pub use inputs::Frame;
pub use stages::StageKind;
