//! collision and out of bound detects

use bevy::prelude::*;

use crate::{
    primitives::{CustomTransform, MkRect},
    util::rotate_vec2,
    world::WorldSize,
};
use crate::primitives::WrapRadian;
// perfecting out of bound not the priority, will be when polishing

/// check if a Sprite is out-of-bounds by checking it's 4 corners
///
/// `sprite_size` takes a full width * length of a Sprite,
///
/// `pos` has center point at the center.
/// ### Performance
/// slow if close to the border
pub fn out_of_bounds(bound: &WorldSize, sprite: MkRect, rotation: Quat) -> bool {
    // if not near the border, return without redundant operations
    if !out_of_bound_point(
        bound,
        MkRect {
            center: sprite.center,
            dimensions: sprite.dimensions.large_bounding_box(),
        },
    ) {
        return false;
    }

    let world_bound = bound.to_rect(vec2(0.0, 0.0));

    sprite
        .get_relative_corners()
        .map(|corner| rotate_vec2(corner, rotation.wrap_radian()))
        .any(|corner| {
            let corner = sprite.center + corner;
            !world_bound.contains(corner)
        })
}

/// faster version of out_of_bounds with a rect, no rotation
pub(crate) fn out_of_bound_point(bound: &WorldSize, rect: MkRect) -> bool {
    let world_bound: MkRect = MkRect {
        center: Vec2::ZERO,
        dimensions: bound.get_size().into(),
    };

    rect.get_corners()
        .iter()
        .any(|corner| !world_bound.contains(*corner))
}

/// primarily used for rig spawning,
/// returns false if intersects
/// ### length is added half
pub(crate) fn square_does_not_intersects(
    center: Vec2,
    mut length: f32,
    other_center: Vec2,
    mut other_length: f32,
) -> bool {
    length *= 1.5;
    other_length *= 1.5;
    // less boilerplate
    length /= 2.0;
    other_length /= 2.0;

    let left = center.x - length;
    let right = center.x + length;
    let top = center.y + length;
    let bottom = center.y - length;

    let other_left = other_center.x - other_length;
    let other_right = other_center.x + other_length;
    let other_top = other_center.y + other_length;
    let other_bottom = other_center.y - other_length;

    right <= other_left || left >= other_right || top <= other_bottom || bottom >= other_top
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
    let sweep = transform.speed.get_raw(); // * delta_seconds;  // mk48 uses a meter -> actual speed
    let other_sweep = other_transform.speed.get_raw(); // * delta_seconds;

    let d2 = transform
        .position
        .0
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