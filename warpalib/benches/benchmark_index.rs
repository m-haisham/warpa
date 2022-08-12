use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_pickle::Value;
use warpalib::Index;

fn criterion_benchmark(c: &mut Criterion) {
    let value = Value::List(vec![Value::List(vec![Value::I64(24), Value::I64(10856)])]);
    c.bench_with_input(BenchmarkId::new("from_value", &value), &value, |b, v| {
        b.iter(|| Index::from_value(v.clone(), None))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
