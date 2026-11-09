//! The resource pack formats vanilla doesn't read (OptiFine's, Polytone's), on the engine's
//! extension points (`horizon_client_render::pack_hooks`). Platform code (docs/layers.md): the
//! platform's game installs it ([`install`]) before the engine starts.

// The formats name the engine's modules as `crate::<module>`, as they did when they lived in it.
#[allow(unused_imports)]
use horizon_client_render::*;

pub mod pack_formats;

pub use pack_formats::install;
