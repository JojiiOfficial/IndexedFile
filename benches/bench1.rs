use std::sync::Arc;
use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use indexed_file::bufreader::IndexedBufReader;
use indexed_file::index::Index;
use indexed_file::string::IndexedString;
use indexed_file::{File, Indexable, ReadByLine};
use rand::distributions::Uniform;
use rand::Rng;
use std::fs;
use std::io::{BufRead, BufReader};

fn in_memory_random_lines_bench(c: &mut Criterion) {
    c.bench_function("read random lines in memory", |b| {
        b.iter_custom(|iters| {
            // Read into memory
            let s = fs::read_to_string("./testfiles/LICENSE").unwrap();
            let mut mem_reader = IndexedString::new_raw(&s).unwrap();

            let lines: Vec<_> = rand::thread_rng()
                .sample_iter(Uniform::new(0, mem_reader.total_lines() - 1))
                .take(mem_reader.total_lines())
                .collect();

            let start = Instant::now();

            for _i in 0..iters {
                for line in &lines {
                    mem_reader.read_line(black_box(*line)).unwrap();
                }
            }

            start.elapsed()
        });
    });
}

fn random_lines_bench(c: &mut Criterion) {
    c.bench_function("read random lines", |b| {
        b.iter_custom(|iters| {
            let mut file = File::open_raw("./testfiles/LICENSE").unwrap();

            let lines: Vec<_> = rand::thread_rng()
                .sample_iter(Uniform::new(0, file.total_lines() - 1))
                .take(file.total_lines())
                .collect();

            let start = Instant::now();

            for _i in 0..iters {
                for line in &lines {
                    file.read_line(black_box(*line)).unwrap();
                }
            }

            start.elapsed()
        });
    });
}

fn sequencial_bench(c: &mut Criterion) {
    c.bench_function("read sequential", |b| {
        b.iter_custom(|iters| {
            let mut file = File::open_raw("./testfiles/LICENSE").unwrap();

            let start = Instant::now();

            let mut buff = Vec::new();
            for _i in 0..iters {
                for line in 0..file.total_lines() - 1 {
                    file.read_line_raw(black_box(line), &mut buff).unwrap();
                }
            }

            start.elapsed()
        });
    });
}

fn sequencial_in_memory_bench(c: &mut Criterion) {
    c.bench_function("read sequential in memory", |b| {
        b.iter_custom(|iters| {
            let s = fs::read_to_string("./testfiles/LICENSE").unwrap();
            let mut in_mem_file = IndexedString::new_raw(&s).unwrap();

            let start = Instant::now();

            let mut buff = Vec::new();
            for _i in 0..iters {
                for line in 0..in_mem_file.total_lines() - 1 {
                    in_mem_file
                        .read_line_raw(black_box(line), &mut buff)
                        .unwrap();
                }
            }

            start.elapsed()
        });
    });
}

fn sequencial_bench_async_std(c: &mut Criterion) {
    c.bench_function("read sequential std implementation", |b| {
        b.iter_custom(|iters| {
            let file = fs::File::open("./testfiles/LICENSE").unwrap();
            let mut reader = BufReader::new(file).lines();

            let start = Instant::now();

            for _i in 0..iters {
                while let Some(line) = reader.next() {
                    let line = black_box(line.unwrap());
                }
            }

            start.elapsed()
        });
    });
}

criterion_group!(
    benches,
    in_memory_random_lines_bench,
    random_lines_bench,
    sequencial_bench,
    sequencial_in_memory_bench,
    sequencial_bench_async_std
);
criterion_main!(benches);
