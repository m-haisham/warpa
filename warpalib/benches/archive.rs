use std::io::Cursor;

use criterion::{criterion_group, criterion_main, Criterion};
use warpalib::RenpyArchive;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("archive write", |b| {
        b.iter(|| {
            let mut archive = RenpyArchive::new();
            archive.content.insert_raw("data1", vec![0u8; 255]);
            archive.content.insert_raw("data2", vec![0u8; 255]);
            archive.content.insert_raw("data3", vec![0u8; 255]);

            let mut buffer = Cursor::new(vec![]);
            archive.flush(&mut buffer).unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
