//! ALICE-View × ALICE-Physics Bridge
//!
//! Interactive physics visualization in the renderer.
//! Converts PhysicsWorld body data into renderable geometry for debug overlay.

use alice_physics::{PhysicsWorld, BodyType};

/// Renderable body representation for the viewer.
#[derive(Clone, Debug)]
pub struct RenderBody {
    /// Position in world space (f32 for GPU).
    pub position: [f32; 3],
    /// Linear velocity direction (normalized, for arrow rendering).
    pub velocity_dir: [f32; 3],
    /// Speed magnitude.
    pub speed: f32,
    /// Body radius estimate (for sphere rendering).
    pub radius: f32,
    /// Whether body is static.
    pub is_static: bool,
    /// Body index in physics world.
    pub body_index: usize,
}

/// Extract renderable body data from a physics world.
///
/// Converts all bodies' fixed-point positions and velocities to
/// f32 for GPU rendering.
pub fn extract_render_bodies(world: &PhysicsWorld) -> Vec<RenderBody> {
    world.bodies.iter().enumerate().map(|(i, body)| {
        let (px, py, pz) = body.position.to_f32();
        let (vx, vy, vz) = body.velocity.to_f32();

        let speed = (vx * vx + vy * vy + vz * vz).sqrt();
        let (dx, dy, dz) = if speed > 1e-6 {
            let speed_rcp = 1.0 / speed;
            (vx * speed_rcp, vy * speed_rcp, vz * speed_rcp)
        } else {
            (0.0, 0.0, 0.0)
        };

        let is_static = matches!(body.body_type, BodyType::Static);

        RenderBody {
            position: [px, py, pz],
            velocity_dir: [dx, dy, dz],
            speed,
            radius: 0.5,
            is_static,
            body_index: i,
        }
    }).collect()
}

/// Compute the axis-aligned bounding box of all bodies (for camera framing).
///
/// Returns `(min, max)` corners.
pub fn compute_world_bounds(world: &PhysicsWorld) -> ([f32; 3], [f32; 3]) {
    if world.bodies.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }

    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    for body in &world.bodies {
        let (px, py, pz) = body.position.to_f32();
        let p = [px, py, pz];
        for d in 0..3 {
            if p[d] < min[d] { min[d] = p[d]; }
            if p[d] > max[d] { max[d] = p[d]; }
        }
    }

    (min, max)
}

/// Count bodies by type.
pub struct BodyCounts {
    /// Number of dynamic bodies.
    pub dynamic: usize,
    /// Number of static bodies.
    pub r#static: usize,
    /// Number of kinematic bodies.
    pub kinematic: usize,
    /// Total body count.
    pub total: usize,
}

/// Count physics world body types for HUD display.
pub fn count_bodies(world: &PhysicsWorld) -> BodyCounts {
    let mut dynamic = 0;
    let mut r#static = 0;
    let mut kinematic = 0;
    for body in &world.bodies {
        match body.body_type {
            BodyType::Dynamic => dynamic += 1,
            BodyType::Static => r#static += 1,
            BodyType::Kinematic => kinematic += 1,
        }
    }
    BodyCounts {
        dynamic,
        r#static,
        kinematic,
        total: world.bodies.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alice_physics::PhysicsConfig;

    #[test]
    fn test_extract_render_bodies() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        world.add_body(RigidBody::new_dynamic(
            Vec3Fix::from_int(5, 10, 0),
            Fix128::ONE,
        ));
        world.add_body(RigidBody::new_static(Vec3Fix::ZERO));

        let bodies = extract_render_bodies(&world);
        assert_eq!(bodies.len(), 2);

        // First body at (5, 10, 0)
        assert!((bodies[0].position[0] - 5.0).abs() < 0.01);
        assert!((bodies[0].position[1] - 10.0).abs() < 0.01);
        assert!(!bodies[0].is_static);

        // Second body is static
        assert!(bodies[1].is_static);
    }

    #[test]
    fn test_count_bodies() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        world.add_body(RigidBody::new_dynamic(Vec3Fix::ZERO, Fix128::ONE));
        world.add_body(RigidBody::new_dynamic(Vec3Fix::ZERO, Fix128::ONE));
        world.add_body(RigidBody::new_static(Vec3Fix::ZERO));

        let counts = count_bodies(&world);
        assert_eq!(counts.dynamic, 2);
        assert_eq!(counts.r#static, 1);
        assert_eq!(counts.total, 3);
    }

    #[test]
    fn test_world_bounds() {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        world.add_body(RigidBody::new_static(Vec3Fix::from_int(-5, 0, 0)));
        world.add_body(RigidBody::new_static(Vec3Fix::from_int(10, 20, 5)));

        let (min, max) = compute_world_bounds(&world);
        assert!((min[0] - (-5.0)).abs() < 0.01);
        assert!((max[0] - 10.0).abs() < 0.01);
        assert!((max[1] - 20.0).abs() < 0.01);
    }
}
