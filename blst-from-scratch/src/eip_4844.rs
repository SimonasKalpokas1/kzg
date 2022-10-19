use std::convert::TryInto;
use std::fs::File;
use std::io::Read;

use blst::{
    blst_p1, blst_p1_affine, blst_p1_from_affine, blst_p1_uncompress, blst_p2, blst_p2_affine,
    blst_p2_from_affine, blst_p2_uncompress, BLST_ERROR, blst_p1_compress,
};
use kzg::{FFTSettings, Fr, KZGSettings, Poly, FFTG1};

use crate::types::fft_settings::FsFFTSettings;
use crate::types::fr::FsFr;
use crate::types::g1::FsG1;

use crate::kzg_proofs::g1_linear_combination;
use crate::types::g2::FsG2;
use crate::types::kzg_settings::FsKZGSettings;
use crate::types::poly::FsPoly;
use crate::utils::reverse_bit_order;

// is .h failo

// typedef blst_p1 g1_t;         /**< Internal G1 group element type */
// typedef blst_p2 g2_t;         /**< Internal G2 group element type */
// typedef blst_fr fr_t;         /**< Internal Fr field element type */
// typedef g1_t KZGCommitment;
// typedef g1_t KZGProof;
// typedef fr_t BLSFieldElement;

/**
 * Montgomery batch inversion in finite field
 *
 * @param[out] out The inverses of @p a, length @p len
 * @param[in]  a   A vector of field elements, length @p len
 * @param[in]  len Length
 */

/*
 C_KZG_RET load_trusted_setup(KZGSettings *out, FILE *in) {
  uint64_t n2, i;
  int j; uint8_t c[96];
  blst_p2_affine g2_affine;
  g1_t *g1_projective;

  fscanf(in, "%" SCNu64, &out->length);
  fscanf(in, "%" SCNu64, &n2);

  TRY(new_g1_array(&out->g1_values, out->length));
  TRY(new_g2_array(&out->g2_values, n2));

  TRY(new_g1_array(&g1_projective, out->length));

  for (i = 0; i < out->length; i++) {
    for (j = 0; j < 48; j++) {
      fscanf(in, "%2hhx", &c[j]);
    }
    bytes_to_g1(&g1_projective[i], c);
  }

  for (i = 0; i < n2; i++) {
    for (j = 0; j < 96; j++) {
      fscanf(in, "%2hhx", &c[j]);
    }
    blst_p2_uncompress(&g2_affine, c);
    blst_p2_from_affine(&out->g2_values[i], &g2_affine);
  }

  unsigned int max_scale = 0;
  while (((uint64_t)1 << max_scale) < out->length) max_scale++;

  out->fs = (FFTSettings*)malloc(sizeof(FFTSettings));

  TRY(new_fft_settings((FFTSettings*)out->fs, max_scale));

  TRY(fft_g1(out->g1_values, g1_projective, true, out->length, out->fs));

  TRY(reverse_bit_order(out->g1_values, sizeof(g1_t), out->length));

  free(g1_projective);

  return C_KZG_OK;
} */

pub fn bytes_to_g1(bytes: [u8; 48usize]) -> FsG1 {
    let mut tmp = blst_p1_affine::default();
    let mut g1 = blst_p1::default();
    unsafe {
        if blst_p1_uncompress(&mut tmp, bytes.as_ptr()) != BLST_ERROR::BLST_SUCCESS {
            panic!("blst_p1_uncompress failed");
        }
        blst_p1_from_affine(&mut g1, &tmp);
    }
    FsG1(g1)
}

pub fn bytes_from_g1(out: &mut [u8; 48usize], g1: &FsG1) {
    unsafe{
        blst_p1_compress(out.as_mut_ptr(), &g1.0);
        // nezinau ka .0 daro
    }
  }

pub fn load_trusted_setup(filepath: &str) -> FsKZGSettings {
    let mut file = File::open(filepath).expect("Unable to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read file");

    let mut lines = contents.lines();
    let length = lines.next().unwrap().parse::<usize>().unwrap();
    let n2 = lines.next().unwrap().parse::<usize>().unwrap();

    let mut g2_values: Vec<FsG2> = Vec::new();

    let mut g1_projectives: Vec<FsG1> = Vec::new();

    for _ in 0..length {
        let line = lines.next().unwrap();
        let bytes = (0..line.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&line[i..i + 2], 16).unwrap())
            .collect::<Vec<u8>>();
        let mut bytes_array: [u8; 48] = [0; 48];
        bytes_array.copy_from_slice(&bytes);
        g1_projectives.push(bytes_to_g1(bytes_array));
    }

    for _ in 0..n2 {
        let line = lines.next().unwrap();
        let bytes = (0..line.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&line[i..i + 2], 16).unwrap())
            .collect::<Vec<u8>>();
        let mut bytes_array: [u8; 96] = [0; 96];
        bytes_array.copy_from_slice(&bytes);
        let mut tmp = blst_p2_affine::default();
        let mut g2 = blst_p2::default();
        unsafe {
            if blst_p2_uncompress(&mut tmp, bytes.as_ptr()) != BLST_ERROR::BLST_SUCCESS {
                panic!("blst_p2_uncompress failed");
            }
            blst_p2_from_affine(&mut g2, &tmp);
        }
        g2_values.push(FsG2(g2));
    }

    let mut max_scale: usize = 0;
    while (1 << max_scale) < length {
        max_scale += 1;
    }

    let fs = FsFFTSettings::new(max_scale).unwrap();

    let mut g1_values = fs.fft_g1(&g1_projectives, true).unwrap();

    reverse_bit_order(&mut g1_values);

    FsKZGSettings {
        secret_g1: g1_values,
        secret_g2: g2_values,
        fs,
    }
}

pub fn fr_batch_inv(out: &mut [FsFr], a: &[FsFr], len: usize) {
    let prod: &mut Vec<FsFr> = &mut vec![FsFr::default(); len];
    // let mut inv : &mut FsFr;
    let mut i: usize = 1;

    prod[0] = a[0];

    while i < len {
        prod[i] = a[i].mul(&prod[i - 1]);
        i += 1;
    }

    let inv: &mut FsFr = &mut prod[len - 1].eucl_inverse();

    i = len - 1;
    while i > 0 {
        out[i] = prod[i - 1].mul(inv);
        *inv = a[i].mul(inv);
        i -= 1;
    }
    out[0] = *inv;
}

pub fn bytes_to_bls_field(out: &mut FsFr, bytes: [u8; 32usize]) {
    *out = FsFr::from_scalar(bytes);
}

pub fn vector_lincomb(vectors : Vec<FsFr>, scalars: Vec<FsFr>, n : usize, m : usize) -> Vec<FsFr>{
    let mut tmp: FsFr  = FsFr::default();
    let mut out: Vec<FsFr> = vec![FsFr::zero(); m.try_into().unwrap()];
    for i in 0..n {
      for j in 0..m{
        tmp = scalars[i].mul(&vectors[i * m + j]);
        out[j] = out[j].add(&tmp);
      }
    };
    out
  }

pub fn bytes_from_bls_field(out: &mut [u8; 32usize], fr: FsFr) {
    
    *out = fr.to_scalar();
  }

pub fn g1_lincomb(out: &mut FsG1, points: &[FsG1], scalars: &[FsFr], num_points: usize) {
    g1_linear_combination(out, points, scalars, num_points)
}

pub fn blob_to_kzg_commitment(out: &mut FsG1, blob: &Vec<FsFr>, s: &FsKZGSettings) {
    g1_lincomb(out, &s.secret_g1, blob, s.secret_g2.len());
}

pub fn verify_kzg_proof(
    out: &mut bool,
    polynomial_kzg: &FsG1,
    z: &FsFr,
    y: &FsFr,
    kzg_proof: &FsG1,
    s: &FsKZGSettings,
) {
    *out = s
        .check_proof_single(polynomial_kzg, kzg_proof, z, y)
        .unwrap_or(false)
}

pub fn compute_kzg_proof(out: &mut FsG1, p: &mut FsPoly, x: &FsFr, s: &FsKZGSettings) {
    if p.len() > s.secret_g1.len() {
        return;
    }

    let mut y: FsFr = FsFr::default();
    evaluate_polynomial_in_evaluation_form(&mut y, p, x, s);

    let mut tmp: FsFr;
    let roots_of_unity: &Vec<FsFr> = &s.fs.expanded_roots_of_unity; // gali buti ne tas
    let mut i: usize = 0;
    let mut m: usize = 0;

    let mut q: FsPoly = FsPoly::new(p.len()).unwrap();

    let mut inverses_in: Vec<FsFr> = vec![FsFr::default(); p.len()];
    let mut inverses: Vec<FsFr> = vec![FsFr::default(); p.len()];

    while i < q.len() {
        if x.equals(&roots_of_unity[i]) {
            m = i + 1;
            continue;
        }
        // (p_i - y) / (ω_i - x)
        q.coeffs[i] = p.coeffs[i].sub(&y);
        inverses_in[i] = roots_of_unity[i].sub(x);
        i += 1;
    }

    fr_batch_inv(&mut inverses, &inverses_in, q.len());

    i = 0;
    while i < q.len() {
        q.coeffs[i] = q.coeffs[i].mul(&inverses[i]);
        i += 1;
    }

    if m > 0 {
        // ω_m == x
        q.coeffs[m] = FsFr::zero();
        m -= 1;
        i = 0;
        while i < q.coeffs.len() {
            if i == m {
                continue;
            }
            // (p_i - y) * ω_i / (x * (x - ω_i))
            tmp = x.sub(&roots_of_unity[i]);
            inverses_in[i] = tmp.mul(x);
            i += 1;
        }
        fr_batch_inv(&mut inverses, &inverses_in, q.coeffs.len());
        i = 0;
        while i < q.coeffs.len() {
            tmp = p.coeffs[i].sub(&y);
            tmp = tmp.mul(&roots_of_unity[i]);
            tmp = tmp.mul(&inverses[i]);
            q.coeffs[m] = q.coeffs[m].add(&tmp);
            i += 1;
        }
    }

    g1_lincomb(out, &s.secret_g1, &q.coeffs, q.coeffs.len());
}

pub fn evaluate_polynomial_in_evaluation_form(
    out: &mut FsFr,
    p: &FsPoly,
    x: &FsFr,
    s: &FsKZGSettings,
) {
    let mut tmp: FsFr;

    let mut inverses_in: Vec<FsFr> = vec![FsFr::default(); p.len()];
    let mut inverses: Vec<FsFr> = vec![FsFr::default(); p.len()];
    let mut i: usize = 0;
    let mut roots_of_unity: Vec<FsFr> = s.fs.expanded_roots_of_unity.clone();

    reverse_bit_order(& mut roots_of_unity);

    while i < p.len() {
        if x.equals(&roots_of_unity[i]) {
            *out = p.get_coeff_at(i);
            return;
        }

        inverses_in[i] = x.sub(&roots_of_unity[i]);
        i += 1;
    }
    fr_batch_inv(&mut inverses, &inverses_in, p.len());

    *out = FsFr::zero();
    i = 0;
    while i < p.len() {
        tmp = inverses[i].mul(&roots_of_unity[i]);
        tmp = tmp.mul(&p.coeffs[i]);
        *out = out.add(&tmp);
        i += 1;
    }
    tmp = FsFr::from_u64(p.len().try_into().unwrap());
    *out = out.div(&tmp).unwrap();
    tmp = x.pow(p.len());
    tmp = tmp.sub(&FsFr::one());
    *out = out.mul(&tmp);
}

pub fn compute_powers(base: &FsFr, num_powers: usize) -> Vec<FsFr> {
    let mut powers: Vec<FsFr> = vec![FsFr::default(); num_powers];
    powers[0] = FsFr::one();
    for i in 1..num_powers {
        powers[i] = powers[i - 1].mul(base);
    }
    powers
}

// pub fn bytes_to_bls_field<TFsFr: FsFr>(bytes: &[u8]) -> TFsFr {
//     TFsFr::from_scalar(bytes)
// }

// kompiliavimo komanda: $env:CARGO_INCREMENTAL=0; cargo build
// kita: cargo test --package blst_from_scratch --test eip_4844 -- tests::test_g1_lincomb --exact --nocapture