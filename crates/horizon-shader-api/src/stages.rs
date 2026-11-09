//! The stages a pack can fill and the WGSL around its files: the stage interface is the
//! renderer's (`horizon_render_pipeline::stages`, re-exported here); this module puts a pack's
//! files and declarations between its prelude and its entry points.
pub use horizon_render_pipeline::stages::*;

use crate::package::Pack;

/// The declarations of a pack's `[[textures]]`, part of every stage's prelude.
pub fn texture_declarations(textures: &[crate::manifest::PackTexture]) -> String {
    let mut out = String::new();
    for (i, t) in textures.iter().enumerate() {
        out += &format!(
            "@group({GROUP}) @binding({}) var {}: texture_2d<f32>;\n\
             @group({GROUP}) @binding({}) var {}_sampler: sampler;\n",
            TEXTURE_BINDING + i as u32,
            t.id,
            TEXTURE_SAMPLER_BINDING + i as u32,
            t.id
        );
    }
    out
}

/// A stage's WGSL, and where each part of it came from.
#[derive(Debug)]
pub struct Composed {
    pub text: String,
    /// (the file, or `<prelude>` / `<entries>`, its byte range in `text`).
    pub parts: Vec<(String, std::ops::Range<usize>)>,
}

impl Composed {
    /// The file, line and column (1-based) of a byte offset in `text`.
    pub fn locate(&self, offset: usize) -> (&str, usize, usize) {
        let (file, range) = self
            .parts
            .iter()
            .find(|(_, r)| r.contains(&offset))
            .or(self.parts.last())
            .expect("a composed stage has parts");
        let before = &self.text[range.start..offset.clamp(range.start, range.end)];
        let line = before.matches('\n').count() + 1;
        let column = before.len() - before.rfind('\n').map_or(0, |i| i + 1) + 1;
        (file, line, column)
    }
}

/// The declarations of a pack's post targets, part of the post stage's prelude.
pub fn post_target_declarations(manifest: &crate::manifest::Manifest) -> String {
    manifest
        .post_targets()
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            format!(
                "@group({GROUP}) @binding({}) var {name}: texture_2d<f32>;\n",
                POST_TARGET_BINDING + i as u32
            )
        })
        .collect()
}

/// The WGSL of the stage `kind` whose own file is `file`: the prelude, the
/// pack's shared files in order, the file, then the entry points, all of the pack's line.
pub fn compose(pack: &Pack, kind: StageKind, file: &str) -> Composed {
    let api = pack.manifest.api;
    let mut composed = Composed {
        text: String::new(),
        parts: Vec::new(),
    };
    let mut push = |name: &str, text: &str| {
        let start = composed.text.len();
        composed.text += text;
        if !text.ends_with('\n') {
            composed.text.push('\n');
        }
        composed
            .parts
            .push((name.to_owned(), start..composed.text.len()));
    };
    let mut prelude = kind.prelude(api) + &texture_declarations(&pack.manifest.textures);
    if kind == StageKind::Post {
        prelude += &post_target_declarations(&pack.manifest);
    }
    push("<prelude>", &prelude);
    for shared in pack.manifest.shared.iter().filter(|s| *s != file) {
        push(shared, pack.source(shared).unwrap_or_default());
    }
    // The stage's own extra files, between the shared ones and its file.
    let m = &pack.manifest;
    let stage = match kind {
        StageKind::Sky => m.sky.as_ref(),
        StageKind::Material => m.material.as_ref(),
        StageKind::Vertex => m.vertex.as_ref(),
        StageKind::Weather => m.weather.as_ref(),
        StageKind::Post => m.post.iter().find(|s| s.file == file),
        StageKind::Tonemap => m.tonemap.as_ref(),
    };
    for extra in stage.into_iter().flat_map(|s| &s.files) {
        if extra != file && !m.shared.contains(extra) {
            push(extra, pack.source(extra).unwrap_or_default());
        }
    }
    push(file, pack.source(file).unwrap_or_default());
    // They draw terrain together: the vertex stage's function is in the material's module.
    if kind == StageKind::Material
        && let Some(vertex) = &m.vertex
    {
        for extra in vertex.files.iter().chain(std::iter::once(&vertex.file)) {
            let own = stage.is_some_and(|s| s.files.contains(extra));
            if !own && !m.shared.contains(extra) {
                push(extra, pack.source(extra).unwrap_or_default());
            }
        }
    }
    if kind == StageKind::Material && m.displacement.is_some() {
        push(
            "<entries>",
            &(kind.entries(api).to_owned() + DISPLACEMENT_ENTRIES),
        );
    } else {
        push("<entries>", kind.entries(api));
    }
    composed
}
