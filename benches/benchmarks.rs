use sandbox::engine::{Sandbox, UserEvent, Kind};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("sand 20", |b| b.iter(|| {
        let mut sandbox = Sandbox::new(50, 50);
        sandbox.tick(Some(UserEvent {
            x: 0, y: 0,
            kind: Kind::Sand,
            size: 20
        }));
        for _ in 0..20 {
            sandbox.tick(None);
        }
    }));

    c.bench_function("sand 50", |b| b.iter(|| {
        let mut sandbox = Sandbox::new(50, 50);
        sandbox.tick(Some(UserEvent {
            x: 0, y: 0,
            kind: Kind::Sand,
            size: 50
        }));
        for _ in 0..20 {
            sandbox.tick(None);
        }
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
