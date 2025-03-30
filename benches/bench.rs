#![feature(test)]

extern crate test;
use {
  concurrent_bloom::bloom::{Bloom}, 
  rand::{rng, rngs::ThreadRng, Rng},
  std::sync::atomic::{AtomicBool, Ordering}, 
  test::Bencher,
  rayon::prelude::*, 
};

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
abcdefghijklmnopqrstuvwxyz\
0123456789";
fn random_string(rng: &mut ThreadRng) -> String {
let length = rng.random_range(1..64);
(0..length).map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char).collect()
}

#[bench]
fn bench_bloom(b: &mut Bencher) {
  let mut r: ThreadRng = rng();
  let items: Vec<String> = (0..2000).map(|_| random_string(&mut r)).collect();
  let failed  = AtomicBool::new(false);
  b.iter(|| {
    let bloom: Bloom<String> = Bloom::new(2100, 0.1);
    items.par_iter().for_each(|item| {
      bloom.insert(item);
      if !bloom.contains(item) {
        failed.store(true, Ordering::Relaxed);
      }
    });
  });
  assert!(!failed.load(Ordering::Relaxed));
}