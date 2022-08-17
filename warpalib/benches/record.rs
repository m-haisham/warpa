use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_pickle::Value;
use warpalib::{Record, RpaError};

fn criterion_benchmark(c: &mut Criterion) {
    let value = Value::List(vec![Value::List(vec![Value::I64(24), Value::I64(10856)])]);
    c.bench_with_input(BenchmarkId::new("from_value", "()"), &value, |b, v| {
        b.iter(|| {
            Record::from_value(v.clone(), None)?;
            Ok::<(), RpaError>(())
        })
    });

    let value_with_prefix = Value::List(vec![Value::List(vec![
        Value::I64(24),
        Value::I64(10856),
        Value::Bytes(vec![7u8; 50000]),
    ])]);
    c.bench_with_input(
        BenchmarkId::new("from_value with prefix", "()"),
        &value_with_prefix,
        |b, v| b.iter(|| Record::from_value(v.clone(), None)),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
