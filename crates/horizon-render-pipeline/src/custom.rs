//! A pack's custom uniforms: values the pack computes on the CPU each frame from the game's own
//! uniforms, which the renderer then hands its programs after the frame's. The renderer only
//! drives the evaluator ([`CustomUniforms`]); the pack format supplies it (`horizon-shaders`'
//! `Custom` reads `shaders.properties`' expressions).

/// A value: up to 16 components (matrices only as names, for `m.column.row`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Val {
    pub v: [f64; 16],
    /// Components: 1 for a scalar.
    pub n: usize,
}

impl Val {
    pub fn scalar(x: f64) -> Self {
        let mut v = [0.0; 16];
        v[0] = x;
        Self { v, n: 1 }
    }
    pub fn vector(c: &[f64]) -> Self {
        let mut v = [0.0; 16];
        v[..c.len()].copy_from_slice(c);
        Self { v, n: c.len() }
    }
}

/// What the renderer asks of a pack's custom uniforms. Called once per frame of a pack, never on
/// vanilla's path.
pub trait CustomUniforms: Send + Sync {
    /// The uniforms' names and declared types, in the order [`CustomUniforms::eval`] returns
    /// their values.
    fn names(&self) -> Vec<(String, String)>;

    /// Evaluates everything for a frame `dt` seconds after the last; `builtin` gives the values
    /// of the game's uniforms and constants. Returns the uniforms' values, in `names` order.
    fn eval(&mut self, dt: f64, builtin: &dyn Fn(&str) -> Option<Val>) -> Vec<Val>;

    /// Forgets what smoothing keeps: each smoothed value starts again from its next value.
    fn restart_smoothing(&mut self);

    /// Called before each frame's `eval`, with whether the camera's biome is known.
    fn world_frame(&mut self, biome_known: bool);

    /// Forgets the world's frames, as a pack just loaded has seen none.
    fn forget_world(&mut self);

    /// Restarts smoothing and randomness for a frame of the world's tick `tick`, so a frame
    /// drawn again at that tick evaluates the same.
    fn restart_at(&mut self, tick: u64);

    /// The problems found so far (expressions that did not parse, unknown names), emptied.
    fn take_notes(&mut self) -> Vec<String>;
}
