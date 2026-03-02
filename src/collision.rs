// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

// copied from https://github.com/SoftbearStudios/mk48/tree/main/server/src/collision.rs with minor modifications
// TODO work on collision

use crate::primitives::CustomTransform;
use bevy::prelude::Vec2;

/// sat_collision performs continuous rectangle-based separating axis theorem collision.
pub fn sat_collision(
    mut transform: CustomTransform,
    mut dimensions: Vec2,
    radius: f32,
    mut other_transform: CustomTransform,
    mut other_dimensions: Vec2,
    other_radius: f32,
) -> bool {
    let sweep = transform.speed.0; // * delta_seconds;  // mk48 uses a meter -> actual speed
    let other_sweep = other_transform.speed.0; // * delta_seconds;

    let d2 = transform
        .position.0
        .distance_squared(other_transform.position.0);
    let r2 = (radius + other_radius + sweep + other_sweep).powi(2);
    if d2 > r2 {
        return false;
    }

    let axis_normal = transform.rotation.to_vec();
    let other_axis_normal = other_transform.rotation.to_vec();

    transform.position.0 += axis_normal * (sweep * 0.5);
    other_transform.position.0 += other_axis_normal * (other_sweep * 0.5);

    dimensions.x += sweep;
    other_dimensions.x += other_sweep;

    // Make math easier later on
    other_dimensions *= 0.5;
    dimensions *= 0.5;

    sat_collision_half(
        transform.position.0,
        other_transform.position.0,
        axis_normal,
        other_axis_normal,
        dimensions,
        other_dimensions,
    ) && sat_collision_half(
        other_transform.position.0,
        transform.position.0,
        other_axis_normal,
        axis_normal,
        other_dimensions,
        dimensions,
    )
}

/// sat_collision_half performs half an SAT test (checks angles of one of two rectangles).
fn sat_collision_half(
    position: Vec2,
    other_position: Vec2,
    mut axis_normal: Vec2,
    other_axis_normal: Vec2,
    dimensions: Vec2,
    other_dimensions: Vec2,
) -> bool {
    let other_axis_tangent = other_axis_normal.perp();

    let other_ps: [Vec2; 4] = [
        other_position
            + other_axis_normal * other_dimensions.x
            + other_axis_tangent * other_dimensions.y,
        other_position + other_axis_normal * other_dimensions.x
            - other_axis_tangent * other_dimensions.y,
        other_position
            - other_axis_normal * other_dimensions.x
            - other_axis_tangent * other_dimensions.y,
        other_position - other_axis_normal * other_dimensions.x
            + other_axis_tangent * other_dimensions.y,
    ];

    for f in 0..4 {
        let dimension = if f % 2 == 0 {
            dimensions.x
        } else {
            dimensions.y
        };

        let dot = position.dot(axis_normal);

        // Dimension is always positive, so min < max.
        let min = dot - dimension;
        let max = dot + dimension;

        let mut less = true;
        let mut greater = true;

        for other_p in other_ps {
            let d = other_p.dot(axis_normal);
            less &= d < min;
            greater &= d > max;
        }

        if less || greater {
            return false;
        }

        // Start over with next axis.
        axis_normal = axis_normal.perp();
    }

    true
}

#[cfg(test)]
mod tests {
    use bevy::math::vec2;

    use crate::primitives::{Position, Radian, Speed};

    use super::*;
    #[test]
    fn sat_test() {
        let transform = CustomTransform {
            speed: Speed(0.0),
            position: Position(vec2(0.0, 0.0)),
            rotation: Radian::from_deg(90.0),
            reversed: false
        };
        let dimensions = vec2(10.0, 10.0);
        let radius = 0.0;

        let other_transform = CustomTransform {
            speed: Speed(0.0),
            position: Position(vec2(4.0, 4.0)),
            rotation: Radian::from_deg(90.0),
            reversed: false,
        };
        let other_dimensions = vec2(10.0, 10.0);
        let other_radius = 0.0;

        assert!(sat_collision(transform, dimensions, radius, other_transform, other_dimensions, other_radius));
    }
}