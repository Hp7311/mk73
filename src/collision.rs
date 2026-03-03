//! collision and out of bound detects

use bevy::prelude::*;

use crate::{primitives::{CustomTransform, WidthHeight}, util::rotate_vec2, world::WorldSize};

// TODO test
/// check if a Sprite is out-of-bounds by checking it's 4 corners
/// ### Performance
/// slow if close to the border
pub(crate) fn out_of_bounds(
    bound: &WorldSize,
    sprite_size: WidthHeight,
    pos: Vec2,
    rotation: Quat
) -> bool {
    // if not near the border, return without redundant operations
    if !near_bound_coarse(sprite_size, pos, bound) {
        return false;
    }

    let world_bound = Rect::new(
        -bound.0.width / 2.0,
        -bound.0.height / 2.0, 
        bound.0.width / 2.0,
        bound.0.height / 2.0
    );
    let half_size = vec2(sprite_size.width / 2.0, sprite_size.height / 2.0);

    // relative to centre of sprite
    let corners = [
        vec2(-half_size.x, -half_size.y),
        vec2(half_size.x, -half_size.y),
        vec2(half_size.x, half_size.y),
        vec2(-half_size.x, half_size.y)
    ];

    if corners
        .iter()
        .map(|corner| rotate_vec2(*corner, rotation))
        .any(|corner| {
            let in_world_pos = pos + corner;
            !world_bound.contains(in_world_pos)
        })
    {
        return true;
    }
    
    false
}

/// determine whether perform slow trignometry to calculate out_of_bound
/// 
/// `corners` are relative to `pos`
/// ### Implementation
/// given a rectangle, a square of side length longer_side ^ 2 will always cover the entirety of the rectangle
fn near_bound_coarse(sprite_size: WidthHeight, pos: Vec2, bound: &WorldSize) -> bool {
    let longer_sprite_side = if sprite_size.width < sprite_size.height {
        sprite_size.height
    } else {
        sprite_size.width
    } * 2.0;

    out_of_bound_no_rotation(bound, WidthHeight { width: longer_sprite_side, height: longer_sprite_side }, &pos)
}

/// faster version of out_of_bounds with a point, no rotation
pub(crate) fn out_of_bound_no_rotation(
    bound: &WorldSize,
    sprite_size: WidthHeight,
    pos: &Vec2,
) -> bool {
    let world_bound = Rect::new(
        -bound.0.width / 2.0,
        -bound.0.height / 2.0, 
        bound.0.width / 2.0,
        bound.0.height / 2.0
    );
    let half_size = vec2(sprite_size.width / 2.0, sprite_size.height / 2.0);

    // relative to centre of sprite
    let corners = [
        vec2(-half_size.x, -half_size.y),
        vec2(half_size.x, -half_size.y),
        vec2(half_size.x, half_size.y),
        vec2(-half_size.x, half_size.y)
    ];

    if corners.iter().any(|corner| {
        let in_world_pos = pos + corner;
        !world_bound.contains(in_world_pos)
    }) {
        return true;
    }
    
    false
}


// copied from https://github.com/SoftbearStudios/mk48/tree/main/server/src/collision.rs with minor modifications

// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// sat_collision performs continuous rectangle-based separating axis theorem collision.
pub(crate) fn sat_collision(
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
            position: Position(vec2(3.0, 3.0)),
            rotation: Radian::from_deg(90.0),
            reversed: false,
        };
        let other_dimensions = vec2(10.0, 10.0);
        let other_radius = 0.0;

        assert!(sat_collision(transform, dimensions, radius, other_transform, other_dimensions, other_radius));
    }
}