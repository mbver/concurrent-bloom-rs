use {
  core::fmt, 
  fnv::FnvHasher, 
  rand::{rng, Rng}, 
  serde::{Deserialize, Serialize}, 
  std::{
    cmp, 
    hash::Hasher, 
    marker::PhantomData, 
    sync::atomic::{AtomicU64, Ordering},
  }
};

fn hash<T: AsRef<[u8]>>(input: T, h_key: u64) -> u64 {
  let mut hasher = FnvHasher::with_key(h_key);
  hasher.write(input.as_ref());
  hasher.finish()
}

#[derive(Serialize, Deserialize, Default)]
pub struct Bloom<T: AsRef<[u8]>> {
  n_bits: u64,
  n_bits_set: AtomicU64,
  hash_keys: Vec<u64>,
  bits: Vec<AtomicU64>,
  _marker: PhantomData<T>
}

impl<T: AsRef<[u8]>> fmt::Debug for Bloom<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Bloom {{ num_hash_keys: {}, num_bits: {}, num_bit_sets: {}, bits: ",
      self.hash_keys.len(),
      self.n_bits,
      self.num_bits_set(),
    )?;
    let first = self.bits[0].load(Ordering::Relaxed);
    let first_10_bits = first.reverse_bits() >> 54; // Reverse only 10 bits
    write!(f, "{:010b}..", first_10_bits)?;
    write!(f, " }}")
  }
}

impl<T: AsRef<[u8]>> Bloom<T> {
/// Creates a thread-safe Bloom filter with an optimal bit size and number of hash functions  
/// based on the expected number of items and the desired false positive rate.
pub fn new(n_items: usize, false_rate: f64) ->Self{
    let n_items = cmp::max(1, n_items);
    let mut m = (-(n_items as f64)*false_rate.ln()/(2f64.ln()*2f64.ln())).ceil();
    m = cmp::max(1, m as u64) as f64; // make sure m >= 1
    let k = (2f64.ln())*m/(n_items as f64).round();
    let length = (m as u64 + 63)/64; // calculate the length of the AtomicU64 vector
    let mut r = rng();
    let hash_keys: Vec<u64> = (0..k as usize).map(|_| r.random()).collect();
    Bloom { 
      n_bits: length*64, 
      n_bits_set: AtomicU64::new(0),
      hash_keys,
      bits: (0..length).map(|_| AtomicU64::new(0)).collect(),
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
    true
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

#[cfg(test)]
mod test {
    use {
      super::Bloom, 
      rand::{rng, rngs::ThreadRng, Rng}, 
      rayon::prelude::*, 
      std::sync::atomic::{AtomicU64, Ordering},
    };
  #[test]
  fn test_bloom_constructor() {
    let bloom: Bloom<String> = Bloom::new(0, 0.1);
    assert_eq!(bloom.n_bits, 64);
    assert_eq!(bloom.hash_keys.len(), 3);

    let bloom: Bloom<String> = Bloom::new(10, 0.1);
    assert_eq!(bloom.n_bits, 64);
    assert_eq!(bloom.hash_keys.len(), 3);

    let bloom: Bloom<String> = Bloom::new(100, 0.1);
    assert_eq!(bloom.n_bits, 512);
    assert_eq!(bloom.hash_keys.len(), 3);
  }
  #[test]
  fn test_bloom_hash_keys_randomness() {
    let mut bloom1: Bloom<String> = Bloom::new(10, 0.1);
    let mut bloom2: Bloom<String> = Bloom::new(10, 0.1);
    assert_eq!(bloom1.hash_keys.len(), bloom2.hash_keys.len());
    bloom1.hash_keys.sort_unstable();
    bloom2.hash_keys.sort_unstable();
    assert_ne!(bloom1.hash_keys, bloom2.hash_keys);
  }
  const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                         abcdefghijklmnopqrstuvwxyz\
                         0123456789";
  fn random_string(rng: &mut ThreadRng) -> String {
    let length = rng.random_range(1..64);
    (0..length).map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char).collect()
  }
  #[test]
  fn test_bloom_insert_contains() {
    let bloom: Bloom<String> = Bloom::new(2100, 0.1);
    println!("{:?}", bloom);
    assert_eq!(10112, bloom.n_bits);
    assert_eq!(3, bloom.hash_keys.len());
    let mut r = rng();
    let items: Vec<String> = (0..2000).map(|_| random_string(&mut r)).collect();

    let false_positives = AtomicU64::new(0);
    items.par_iter().for_each(|item| {
      if bloom.contains(&item) {
        false_positives.fetch_add(1, Ordering::Relaxed);
      }
      bloom.insert(&item);
      assert!(bloom.contains(&item));
    });
    assert!(
      false_positives.load(Ordering::Relaxed) < 200, 
      "false_positive: {}", false_positives.load(Ordering::Relaxed));
    // test false_positives more intensively
    false_positives.store(0, Ordering::Relaxed);
    (0..10000).for_each(|_| {
      let item = random_string(&mut r);
      if bloom.contains(&item) {
        false_positives.fetch_add(1, Ordering::Relaxed);
      }
    });
    assert!(false_positives.load(Ordering::Relaxed) < 2000, 
    "false_positive: {}", false_positives.load(Ordering::Relaxed));
  }
}

