use casper_node::types::{from_bytes_block_map, to_bytes_block_map, Block};
use casper_types::{
    bytesrepr::{FromBytes, ToBytes},
    testing::TestRng,
};
use criterion::{
    black_box, criterion_group, criterion_main,
    measurement::{Measurement, WallTime},
    Bencher, BenchmarkGroup, Criterion, Throughput,
};

fn block_roundtrip(block: &Block) {
    let bytes = to_bytes_block_map(black_box(block));
    let _deserialized = from_bytes_block_map(black_box(&bytes));
}

fn bytesrepr_block_roundtrip(block: &Block) {
    let bytes = block.to_bytes().unwrap();
    let _deserialized = Block::from_bytes(black_box(&bytes)).unwrap();
}

fn serialization_bench(c: &mut Criterion) {
    let mut rng = TestRng::from_seed([1u8; 16]);
    let block = black_box(Block::random(&mut rng));

    c.bench_function("block_roundrip_with_bytesrepr", |b| b.iter(|| bytesrepr_block_roundtrip(black_box(&block))));
    c.bench_function("block_roundrip_with_field_map", |b| b.iter(|| block_roundtrip(black_box(&block))));
}

criterion_group!(benches, serialization_bench);
criterion_main!(benches);
