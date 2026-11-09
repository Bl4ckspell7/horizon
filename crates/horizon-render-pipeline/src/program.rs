//! A translated program as the renderer builds its pipelines from it: its naga modules and the
//! interface they expect (uniforms, samplers, images, outputs), and what the renderer offers a
//! program's vertex stage (`GbuffersInputs`, `GeometryInputs`). `horizon-shaders` translates a
//! `shaders/` pack's GLSL into these.

/// What a program's stages need from the game.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interface {
    /// The uniform block at binding 0: its size and each value's name, GLSL type and offset.
    /// Ours come first: `hz_target` (the target's width, height, 1/width, 1/height), and the
    /// fixed-function fog the compatibility `gl_Fog` reads (OQ-28): `hz_fog_color` (vanilla's fog
    /// colour) and `hz_fog` (density, start, end, 1 / (end - start)), then `hz_depth_linear`:
    /// 1 where `shadowtex0`, `shadowtex1`, `shadow` and `watershadow` (x to w) read through a
    /// plain sampler filter linearly, 0 where they take the nearest texel.
    pub uniform_size: u32,
    pub uniforms: Vec<Uniform>,
    /// std140 initial contents; runtime values overwrite their members each frame.
    pub initial_values: Vec<u8>,
    /// Each sampler: texture at `binding`, sampler object at `binding + 1`.
    pub samplers: Vec<Sampler>,
    /// The fragment stage's output locations.
    pub outputs: Vec<u32>,
    /// A gbuffers program reads `DRAW_UNIFORMS`: the block at set 2, binding 0, set per draw.
    pub draw: bool,
    /// Storage images (`colorimg4`...), after the samplers.
    pub images: Vec<Image>,
    /// The SSBOs' bindings in set 4 (`bufferObject.<n>`).
    pub buffers: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub name: String,
    /// `image2D`, `uimage3D`...
    pub ty: String,
    /// The format qualifier (`rgba16f`).
    pub format: String,
    /// `readonly`, `writeonly`, empty for both, or `atomic` for an image an atomic function
    /// uses.
    pub access: String,
    pub binding: u32,
    /// A compute program's copy of image `before` as it was before the dispatch, which its
    /// loads read (see `ordering`).
    pub before: Option<String>,
}

/// A compute program (`deferred4_a.csh`): its module, what it reads and writes, and how many
/// work groups it runs: `workGroups`, or `workGroupsRender` times the target's size over the
/// local size (1, 1 when neither is set).
#[derive(Clone)]
pub struct ComputeProgram {
    pub module: naga::Module,
    pub interface: Interface,
    /// Non-fatal translation diagnostics, located in the authored source.
    pub notes: Vec<Error>,
    pub work_groups: Option<[u32; 3]>,
    pub work_groups_render: [f32; 2],
}

/// The uniforms a gbuffers program reads per draw (5.3's "D"), in a block of their own
/// (`HzDraw`, std140: four ints, a vec4; 32 bytes) the game sets for each draw.
pub const DRAW_UNIFORMS: [(&str, &str); 5] = [
    ("renderStage", "int"),
    ("entityId", "int"),
    ("blockEntityId", "int"),
    ("currentRenderedItemId", "int"),
    ("entityColor", "vec4"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Uniform {
    pub name: String,
    /// The type the pack declared (`float`, `ivec2`, `mat4`; `bool` is stored as an `int`).
    pub ty: String,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sampler {
    pub name: String,
    /// The type the pack declared: `sampler2D`, `sampler2DShadow`, `usampler2D`, `sampler3D`...
    pub ty: String,
    pub binding: u32,
}

/// A translation diagnostic at a pack file and line.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub file: String,
    pub line: u32,
    pub message: String,
}

#[derive(Clone)]
pub struct Program {
    pub vertex: naga::Module,
    pub fragment: naga::Module,
    pub interface: Interface,
    /// Non-fatal translation diagnostics, located in the authored source.
    pub notes: Vec<Error>,
}

/// Where the program draws, which decides the conventions of 5.5.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// Pack buffers: rows stored as GL stores them.
    Buffers,
    /// The window (`final`).
    Window,
}

/// What a gbuffers-style program's vertex stage reads. The game writes it: it knows its vertex
/// formats, the translator only places it.
#[derive(Clone, Debug, Default)]
pub struct GbuffersInputs {
    /// GLSL declarations: the vertex inputs and any bindings of set 1.
    pub declarations: String,
    /// GLSL statements that set `hz_Vertex`, `hz_Color`, `hz_MultiTexCoord0`,
    /// `hz_MultiTexCoord1` and `hz_Normal` (the compatibility inputs), run first in `main`.
    pub body: String,
    /// The value of each attribute a pack may declare (`mc_Entity`, `vaPosition`...), as a GLSL
    /// expression of the declared type's size; an attribute not listed reads zero. Terrain
    /// also supplies `mc_chunkFade`, an implicit float input rather than a declared attribute.
    pub attributes: Vec<(String, String)>,
    /// GLSL statements run last in `main`, with `gl_Position` in wgpu's clip space.
    pub after: String,
    /// The alpha test applied to output 0 (`gl_FragData[0]`, `gl_FragColor`), the loader's
    /// (3.2 `alphaTest.<program>`): `None` keeps every fragment.
    pub alpha_test: Option<AlphaTest>,
    /// The program may read `currentRenderedItemId` without declaring it, as Iris 1.11.7
    /// lets the hand's (observed with probe packs; terrain's may not).
    pub item_id_declared: bool,
    /// The depth test sees the far plane (window depth 1) unless the program writes
    /// `gl_FragDepth`, while `gl_FragCoord.z` stays the geometry's own: the sky shows only where
    /// nothing else drew, and reads its own depth as in Iris 1.11.7.
    pub far_depth: bool,
    /// `normals` and `specular` are bound as paged maps of the atlas `texture` reads
    /// (`horizon-client-render`'s `paged_maps.rs`), not as images of its layout: the program's
    /// texture calls on them read through `hz_pagedTexture`.
    pub paged_maps: bool,
    /// Translucent terrain: a fragment behind the boats' water patches (`horizonWaterMask`, the
    /// patches' depth, 1 where there are none) is discarded, so no water shows inside a hull
    /// (`RenderTypes.waterMask`), while the frame's depth stays the pack's own.
    pub water_mask: bool,
    /// The shadow pass's translucent terrain: a fragment inside a boat's water patch
    /// (`horizonWaterPatch*`, camera-relative boxes) is discarded, so no water casts its light
    /// into a hull. Not with a geometry stage.
    pub water_patches: bool,
}

/// The sampler of the boats' water patches' depth (`GbuffersInputs::water_mask`).
pub const WATER_MASK: &str = "horizonWaterMask";

/// How many boats' water patches the shadow pass keeps water out of (`GbuffersInputs::water_patches`).
pub const WATER_PATCHES: usize = 16;

/// A fragment stays when its output 0's alpha compares to `reference` as `op` says (a GLSL
/// comparison operator, `true` for GL's `ALWAYS`, `false` for `NEVER`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlphaTest {
    pub op: &'static str,
    pub reference: f32,
}

impl AlphaTest {
    /// `alphaTest.<program>`'s value: `off`, or a GL comparison and a reference.
    pub fn parse(value: &str) -> Option<Option<Self>> {
        let mut words = value.split_whitespace();
        let func = words.next()?;
        if func == "off" {
            return Some(None);
        }
        let op = match func.trim_start_matches("GL_") {
            "NEVER" => "false",
            "LESS" => "<",
            "EQUAL" => "==",
            "LEQUAL" => "<=",
            "GREATER" => ">",
            "NOTEQUAL" => "!=",
            "GEQUAL" => ">=",
            "ALWAYS" => return Some(None),
            _ => return None,
        };
        let reference = words.next()?.parse().ok()?;
        Some(Some(Self { op, reference }))
    }
}

/// How a geometry program's vertex invocations reach their triangle's vertices.
#[derive(Clone, Debug, Default)]
pub struct GeometryInputs {
    /// The game's vertex inputs, as `GbuffersInputs`; their `layout(location = n) in`
    /// declarations become plain globals the pull sets.
    pub inputs: GbuffersInputs,
    /// GLSL statements that set the vertex inputs from vertex `hz_vertex_index` (a `uint`).
    pub pull: String,
    /// The vertex of the `@`th corner of the draw (a `uint` expression, `hz_indices[@]`).
    pub index: String,
}
