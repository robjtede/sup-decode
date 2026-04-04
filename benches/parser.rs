#![allow(dead_code, unused_imports)]

use criterion::{Criterion, criterion_group, criterion_main};

#[path = "../src/segment.rs"]
mod segment;

#[path = "../src/decode/mod.rs"]
mod decode;

fn parse_mummyforced(c: &mut Criterion) {
    let bytes = std::fs::read("data/mummyforced.sup").expect("read data/mummyforced.sup");

    c.bench_function("parse_frames/mummyforced", |b| {
        b.iter(|| {
            let frames = decode::parse_frames(&bytes).expect("parse mummyforced.sup");
            criterion::black_box(frames);
        });
    });
}

criterion_group!(benches, parse_mummyforced);
criterion_main!(benches);
