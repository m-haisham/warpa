use std::{io::Cursor, path::PathBuf};

use criterion::{criterion_group, criterion_main, Criterion};
use warpalib::{Content, RenpyArchive};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("archive write", |b| {
        b.iter(|| {
            let mut archive = RenpyArchive::new();
            archive
                .content
                .insert(PathBuf::from("data1"), Content::Raw(vec![0u8; 255]));
            archive
                .content
                .insert(PathBuf::from("data2"), Content::Raw(vec![0u8; 255]));
            archive
                .content
                .insert(PathBuf::from("data3"), Content::Raw(vec![0u8; 255]));

            let mut buffer = Cursor::new(vec![]);
            archive.flush(&mut buffer).unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
