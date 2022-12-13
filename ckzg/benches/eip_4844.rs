use ckzg::{
    eip_4844::{
        blob_to_kzg_commitment_rust, compute_aggregate_kzg_proof_rust, load_trusted_setup_rust,
        verify_aggregate_kzg_proof_rust,
    },
        fftsettings4844::KzgFFTSettings4844, finite::BlstFr, consts::BlstP1, consts::BlstP2, kzgsettings4844::KzgKZGSettings4844,
        poly::KzgPoly,
};
use criterion::{criterion_group, criterion_main, Criterion};
use kzg_bench::benches::eip_4844::{
    bench_compute_aggregate_kzg_proof, bench_verify_aggregate_kzg_proof,
};

fn bench_compute_aggregate_kzg_proof_(c: &mut Criterion) {
    bench_compute_aggregate_kzg_proof::<BlstFr, BlstP1, BlstP2, KzgPoly, KzgFFTSettings4844, KzgKZGSettings4844>(
        c,
        &load_trusted_setup_rust,
        &compute_aggregate_kzg_proof_rust,
        &[1, 2, 4, 8, 16],
    )
}

fn bench_verify_aggregate_kzg_proof_(c: &mut Criterion) {
    bench_verify_aggregate_kzg_proof::<BlstFr, BlstP1, BlstP2, KzgPoly, KzgFFTSettings4844, KzgKZGSettings4844>(
        c,
        &load_trusted_setup_rust,
        &blob_to_kzg_commitment_rust,
        &compute_aggregate_kzg_proof_rust,
        &verify_aggregate_kzg_proof_rust,
        &[1, 2, 4, 8, 16],
    )
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_compute_aggregate_kzg_proof_, bench_verify_aggregate_kzg_proof_
}

criterion_main!(benches);
