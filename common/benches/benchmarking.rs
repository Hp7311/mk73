use std::hint::black_box;
use bevy::math::{Vec2, vec2};
use criterion::{criterion_group, criterion_main, Criterion};
use common::{collision::{out_of_bound_no_rotation, out_of_bound_point, out_of_bounds}, primitives::{Mk48Rect, Radian}, util::{avaliable_cords, tiles_around_point}, WorldSize};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Collisions");

    let world_size = WorldSize::default();
    group.bench_function("Out of bound no rotation", |b| b.iter(|| {
        out_of_bound_no_rotation(&world_size, black_box(Mk48Rect::from_point([0.0, 0.0])))
    }));
    group.bench_function("Out of bound with point", |b| b.iter(|| {
        out_of_bound_point(&world_size, black_box(Vec2::ZERO))
    }));
    group.bench_function("Out of bound with zero rotation but not near border", |b| b.iter(|| {
        out_of_bounds(&world_size, Mk48Rect::from_point([0.0, 0.0]), Radian(0.0))
    }));
    group.finish();

    let mut group = c.benchmark_group("Selecting tile");
    group.bench_function("Seperate Vec2", |b| b.iter(|| {
        tiles_around_point(black_box(vec2(10.0, 10.0)), black_box(10.0));
    }));
    group.bench_function("Rough square", |b| b.iter(|| {
        avaliable_cords(black_box(vec2(10.0, 10.0)), black_box(10.0))
    }));
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);