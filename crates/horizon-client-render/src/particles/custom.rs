//! The extension point for resource pack formats' own particles: the ways a particle quad turns,
//! and GPU particles (Polytone's, #11486), records a store draws beside the engine's particles,
//! which the platform's formats provide.
use super::{Particle, ParticleLightCache};
use crate::hud::Rect;
use crate::mesher::Vertex;
use glam::{DVec3, Mat3, Quat, Vec3 as FVec3};
use std::collections::HashMap;

/// A GPU particle type's records, drawn beside the engine's own particles.
pub trait GpuParticles: Send + Sync {
    /// Whether it holds custom type `name`'s records.
    fn is(&self, name: &str) -> bool;
    /// Records a spawn from a particle its type set up, at engine tick `now`.
    fn spawn(&mut self, particle: &Particle, now: u64, seed: u64);
    /// Forgets the records whose life is over.
    fn expire(&mut self, now: u64);
    /// How many records are live.
    fn records(&self) -> usize;
    /// The quads of every live record at `now + partial` ticks.
    #[allow(clippy::too_many_arguments)]
    fn render(
        &self,
        atlas: &HashMap<String, (Rect, bool)>,
        camera: DVec3,
        facing: Quat,
        now: u64,
        partial: f32,
        light_cache: &mut ParticleLightCache,
        light_at: &dyn Fn([i32; 3]) -> u8,
        out: &mut Vec<Vertex>,
        additive: &mut Vec<Vertex>,
    );
}

/// How a particle quad turns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Faces the camera.
    #[default]
    Camera,
    /// Turns around Y only.
    AroundY,
    /// Lies flat, facing up.
    Flat,
    /// Stands along its velocity, facing the camera around it.
    Movement,
    /// Fixed in the world's axes (its +Z facing south).
    Fixed,
}

/// The rotation of a quad (corners in its XY plane, facing +Z) at `center` (relative to the
/// camera) moving along `velocity`, rolled by `roll`; `camera` is the camera-facing rotation.
pub fn orient(mode: Orientation, center: FVec3, velocity: FVec3, roll: f32, camera: Quat) -> Quat {
    let rolled = |q: Quat| q * Quat::from_rotation_z(roll);
    match mode {
        Orientation::Camera => rolled(camera),
        Orientation::AroundY => rolled(Quat::from_rotation_y((-center.x).atan2(-center.z))),
        Orientation::Flat => rolled(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Orientation::Fixed => rolled(Quat::IDENTITY),
        Orientation::Movement => {
            let up = velocity.normalize_or_zero();
            let right = up.cross((-center).normalize_or_zero()).normalize_or_zero();
            if right == FVec3::ZERO {
                return rolled(camera);
            }
            Quat::from_mat3(&Mat3::from_cols(right, up, right.cross(up)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientations_turn_the_quad_normal() {
        let normal = |q: Quat| q * FVec3::Z;
        let close = |a: FVec3, b: FVec3| (a - b).length() < 1e-5;
        // Lying flat: facing up.
        assert!(close(
            normal(orient(
                Orientation::Flat,
                FVec3::ONE,
                FVec3::ZERO,
                0.0,
                Quat::IDENTITY
            )),
            FVec3::Y
        ));
        // Around Y: facing the camera horizontally (the camera at the origin, the quad east).
        assert!(close(
            normal(orient(
                Orientation::AroundY,
                FVec3::new(5.0, 3.0, 0.0),
                FVec3::ZERO,
                0.0,
                Quat::IDENTITY
            )),
            -FVec3::X
        ));
        // Along its motion: its up is the velocity, its face toward the camera.
        let q = orient(
            Orientation::Movement,
            FVec3::new(0.0, 0.0, -4.0),
            FVec3::new(0.0, -2.0, 0.0),
            0.0,
            Quat::IDENTITY,
        );
        assert!(close(q * FVec3::Y, -FVec3::Y));
        assert!(close(normal(q), FVec3::Z));
    }
}
