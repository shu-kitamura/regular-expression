use criterion::{Criterion, black_box, criterion_group, criterion_main};
use regex_core::Regex;

fn bench_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile");
    let pattern = "a(b|c|d)*xyz";

    group.bench_function("regex_compile", |b| {
        b.iter(|| {
            let compiled = Regex::new(black_box(pattern), false, false).unwrap();
            black_box(compiled);
        })
    });

    group.finish();
}

fn bench_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("match");
    let input_match = "zzzaacccdddbcdxyzend";
    let input_no_match = "zzzaacccdddbcdxyyend";

    let regex = Regex::new("a(b|c|d)*xyz", false, false).unwrap();

    group.bench_function("regex_match_true", |b| {
        b.iter(|| {
            let matched = regex.is_match(black_box(input_match)).unwrap();
            black_box(matched);
        })
    });

    group.bench_function("regex_match_false", |b| {
        b.iter(|| {
            let matched = regex.is_match(black_box(input_no_match)).unwrap();
            black_box(matched);
        })
    });

    group.finish();
}

fn bench_backreference(c: &mut Criterion) {
    let mut group = c.benchmark_group("backreference");
    let regex = Regex::new("(abc)\\1", false, false).unwrap();

    group.bench_function("regex_backref_match_true", |b| {
        b.iter(|| {
            let matched = regex.is_match(black_box("abcabc")).unwrap();
            black_box(matched);
        })
    });

    group.bench_function("regex_backref_match_false", |b| {
        b.iter(|| {
            let matched = regex.is_match(black_box("abcabd")).unwrap();
            black_box(matched);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_compile, bench_match, bench_backreference);
criterion_main!(benches);
