use std::sync::Arc;
use std::time::Instant;

use async_std::fs;
use async_std::io::prelude::BufReadExt;
use async_std::io::{BufReader, Cursor};
use async_std::stream::StreamExt;
use criterion::async_executor::AsyncStdExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use indexed_file::bufreader::IndexedBufReader;
use indexed_file::index::Index;
use indexed_file::{File, Indexable, ReadByLine};
use rand::distributions::Uniform;
use rand::Rng;

fn in_memory_random_lines_bench(c: &mut Criterion) {
    c.bench_function("read random lines in memory", |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                // Read into memory
                let mut b = BufReader::new(Cursor::new(
                    fs::read_to_string("./testfiles/LICENSE").await.unwrap(),
                ));
                // build index
                let index = Arc::new(Index::build(&mut b).await.unwrap());
                // build in memory indexed file/reader
                let mut mem_reader = IndexedBufReader::new(b, index).unwrap();

                let lines: Vec<_> = rand::thread_rng()
                    .sample_iter(Uniform::new(0, mem_reader.total_lines() - 1))
                    .take(mem_reader.total_lines())
                    .collect();

                let start = Instant::now();

                for _i in 0..iters {
                    for line in &lines {
                        mem_reader.read_line(black_box(*line)).await.unwrap();
                    }
                }

                start.elapsed()
            });
    });
}

fn random_lines_bench(c: &mut Criterion) {
    c.bench_function("read random lines", |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut file = File::open_raw("./testfiles/LICENSE").await.unwrap();

                let lines: Vec<_> = rand::thread_rng()
                    .sample_iter(Uniform::new(0, file.total_lines() - 1))
                    .take(file.total_lines())
                    .collect();

                let start = Instant::now();

                for _i in 0..iters {
                    for line in &lines {
                        file.read_line(black_box(*line)).await.unwrap();
                    }
                }

                start.elapsed()
            });
    });
}

fn sequencial_bench(c: &mut Criterion) {
    c.bench_function("read sequential", |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let mut file = File::open_raw("./testfiles/LICENSE").await.unwrap();

                let start = Instant::now();

                let mut buff = Vec::new();
                for _i in 0..iters {
                    for line in 0..file.total_lines() - 1 {
                        file.read_line_raw(black_box(line), &mut buff)
                            .await
                            .unwrap();
                    }
                }

                start.elapsed()
            });
    });
}

fn sequencial_in_memory_bench(c: &mut Criterion) {
    c.bench_function("read sequential in memory", |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                // Read into memory
                let mut b = BufReader::new(Cursor::new(
                    fs::read_to_string("./testfiles/LICENSE").await.unwrap(),
                ));
                // build index
                let index = Arc::new(Index::build(&mut b).await.unwrap());
                // build in memory indexed file/reader
                let mut in_mem_file = IndexedBufReader::new(b, index).unwrap();

                let start = Instant::now();

                let mut buff = Vec::new();
                for _i in 0..iters {
                    for line in 0..in_mem_file.total_lines() - 1 {
                        in_mem_file
                            .read_line_raw(black_box(line), &mut buff)
                            .await
                            .unwrap();
                    }
                }

                start.elapsed()
            });
    });
}

fn sequencial_bench_async_std(c: &mut Criterion) {
    c.bench_function("read sequential std implementation", |b| {
        b.to_async(AsyncStdExecutor)
            .iter_custom(|iters| async move {
                let file = async_std::fs::File::open("./testfiles/LICENSE")
                    .await
                    .unwrap();
                let mut reader = BufReader::new(file).lines();

                let start = Instant::now();

                for _i in 0..iters {
                    while let Some(line) = reader.next().await {
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
