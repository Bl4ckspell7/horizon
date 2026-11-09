//! The particle engine's insides for tests of the crates above it, the pack formats' particles
//! (feature `test-support`), without making them part of what native mods see.
use crate::hud::Rect;
use crate::particles::custom::CustomParticles;
use crate::particles::{Particle, ParticleEngine, ParticleLevel, ParticleType};
use std::collections::HashMap;
use std::sync::Arc;

/// What tests read and seed of a [`ParticleEngine`].
pub trait EngineInsides {
    /// Particles spawned this tick, before the next tick adds them.
    fn pending(&mut self) -> &mut Vec<Particle>;
    /// The live particles, without the pending ones.
    fn particles_mut(&mut self) -> &mut Vec<Particle>;
    /// Sprite rectangles, by sprite index.
    fn rects(&mut self) -> &mut Vec<Option<Rect>>;
    /// Sprites by name, with whether they're translucent.
    fn atlas(&mut self) -> &mut HashMap<String, (Rect, bool)>;
    /// Vanilla particle types' sprite sets.
    fn sets(&mut self) -> &mut HashMap<ParticleType, u32>;
    /// Each sprite set's sprite indices.
    fn set_sprites(&mut self) -> &mut Vec<Vec<u32>>;
    /// Registers a mod particle's sprite set, with no behaviour of its own.
    fn add_mod_set(&mut self, name: &str, set: u32);
    /// The packs' custom particles the engine reads.
    fn custom(&self) -> Option<Arc<dyn CustomParticles>>;
    /// One tick of one particle.
    fn tick_one(&mut self, p: &mut Particle, level: &ParticleLevel);
}

impl EngineInsides for ParticleEngine {
    fn pending(&mut self) -> &mut Vec<Particle> {
        &mut self.pending
    }
    fn particles_mut(&mut self) -> &mut Vec<Particle> {
        &mut self.particles
    }
    fn rects(&mut self) -> &mut Vec<Option<Rect>> {
        &mut self.rects
    }
    fn atlas(&mut self) -> &mut HashMap<String, (Rect, bool)> {
        &mut self.atlas
    }
    fn sets(&mut self) -> &mut HashMap<ParticleType, u32> {
        &mut self.sets
    }
    fn set_sprites(&mut self) -> &mut Vec<Vec<u32>> {
        &mut self.set_sprites
    }
    fn add_mod_set(&mut self, name: &str, set: u32) {
        self.mod_sets.insert(name.to_owned(), (set, None));
    }
    fn custom(&self) -> Option<Arc<dyn CustomParticles>> {
        self.polytone.clone()
    }
    fn tick_one(&mut self, p: &mut Particle, level: &ParticleLevel) {
        self.tick_particle(p, level);
    }
}

/// What tests read of a [`Particle`].
pub trait ParticleInsides {
    /// Whether a controller (a native mod's or a pack's) ticks it.
    fn has_controller(&self) -> bool;
    /// Whether it is gone.
    fn is_removed(&self) -> bool;
    /// Its sprite's index, when it has one.
    fn sprite_index(&self) -> Option<u32>;
    /// Its light coordinates at `partial` under `sky` and `block` light.
    fn light_at(&self, partial: f32, sky: u32, block: u32) -> (u32, u32);
}

impl ParticleInsides for Particle {
    fn has_controller(&self) -> bool {
        self.controller.is_some()
    }
    fn is_removed(&self) -> bool {
        self.removed
    }
    fn sprite_index(&self) -> Option<u32> {
        match self.sprite {
            crate::particles::Sprite::Index(i) => Some(i),
            _ => None,
        }
    }
    fn light_at(&self, partial: f32, sky: u32, block: u32) -> (u32, u32) {
        self.light_coords(partial, sky, block)
    }
}

/// A particle of `ty` at `p` with no motion, sprite or life of its own.
pub fn blank(ty: ParticleType, p: [f64; 3]) -> Particle {
    Particle::blank(ty, p)
}
