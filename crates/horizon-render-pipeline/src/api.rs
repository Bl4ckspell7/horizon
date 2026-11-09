//! The lines of the WGSL stage interface ([`crate::stages`], [`crate::inputs`]).

/// A shader API line: the stages, their signatures and the inputs a pack's WGSL may read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Api {
    /// Vanilla's gamma values.
    V0_1,
    /// Linear light (docs/design/hdr-linear-light.md). Experimental: checked, not yet drawn.
    V0_2,
}

impl Api {
    /// Every line this build reads and checks, oldest first.
    pub const SERVED: [Api; 2] = [Api::V0_1, Api::V0_2];

    /// The manifest's `api` value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Api::V0_1 => "0.1",
            Api::V0_2 => "0.2",
        }
    }

    pub fn parse(line: &str) -> Option<Api> {
        Self::SERVED.into_iter().find(|a| a.as_str() == line)
    }
}

impl std::fmt::Display for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
