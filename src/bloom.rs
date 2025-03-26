use {
  std::{
    cmp, 
    hash::Hasher, 
    marker::PhantomData, 
    sync::atomic::{AtomicU64, Ordering},
  },
  fnv::FnvHasher,
  rand::{rng, Rng}
};

fn hash<T: AsRef<[u8]>>(input: T, h_key: u64) -> u64 {
  let mut hasher = FnvHasher::with_key(h_key);
  hasher.write(input.as_ref());
  hasher.finish()
}

pub struct Bloom<T: AsRef<[u8]>> {
  n_bits: u64,
  n_bits_set: AtomicU64,
  hash_keys: Vec<u64>,
  bits: Vec<AtomicU64>,
  _marker: PhantomData<T>
}

impl<T: AsRef<[u8]>> Bloom<T> {
  fn new(n_items: usize, false_rate: f64) ->Self{
    let m = (-(n_items as f64)*false_rate.ln()/(2f64.ln()*2f64.ln())).ceil();
    let n_bits = cmp::max(1, m as u64);
    let k = (2f64.ln())*(n_bits as f64)/(cmp::max(n_items, 1) as f64).round();
    let mut r = rng();
    let hash_keys: Vec<u64> = (0..k as usize).map(|_| r.random()).collect();
    Bloom { 
      n_bits: n_bits, 
      n_bits_set: AtomicU64::new(0),
      hash_keys: hash_keys,
      bits: (0..n_bits).map(|_| AtomicU64::new(0)).collect(),
      _marker: PhantomData,
    }
  }
  /// Computes the `u64` index and bitmask for a given input and hash key.
  /// This is used to set or check the bit corresponding to the input.
  fn bit_pos(&self, input: &T, h_key: u64) -> (usize, u64) {
    let p = hash(input, h_key) % self.n_bits;
    let idx = p>>6;
    let mask  = 1u64 << (p &63);
    (idx as usize, mask)
  }
  /// Sets the bit corresponding to the given input and hash key in the Bloom filter.
  fn set_bit(&self, input: &T, h_key: u64) -> bool{
    let (idx, mask) = self.bit_pos(input, h_key);
    let prev = self.bits[idx].fetch_or(mask, Ordering::Relaxed);
    let is_new = prev &mask == 0;
    if is_new {
      self.n_bits_set.fetch_add(1, Ordering::Relaxed);
    }
    is_new
  }

  /// Checks the bit corresponding to the given input and hash key in the Bloom filter.
  fn check_bit(&self, input: &T, h_key: u64) -> bool{
    let (idx, mask) = self.bit_pos(input, h_key);
    let bit = self.bits[idx].load(Ordering::Relaxed) & mask;
    bit > 0
  }
  /// Adds an item to Bloom filter
  pub fn insert(&self, item: &T) {
    for h_key in &self.hash_keys {
      self.set_bit(item, *h_key);
    }
  }
  /// Checks if an item is in Bloom filter
  pub fn contains(&self, item: &T) -> bool {
    for h_key in &self.hash_keys {
      if !self.check_bit(item, *h_key) {
        return false;
      }
    }
    return true;
  }
  // clear all the bits
  pub fn reset(&self) {
    for n in &self.bits{
      n.store(0, Ordering::Relaxed);
    }
  }
  // get the number of bits set
  pub fn num_bits_set(&self) ->u64 {
    self.n_bits_set.load(Ordering::Relaxed)
  }
}