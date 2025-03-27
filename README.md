# Bloom Filter (concurrent-bloom-rs)

A concurrent Bloom filter implementation in Rust, designed for efficiency and thread safety.

## Features
- Lock-free, thread-safe operations
- Optimized bit array and hashing
- Supports: insertion, membership checking, and reset

## Usage

### Creating a Bloom Filter
```rust
let bloom = Bloom::new(1000, 0.01);
```

### Inserting an Item
```rust
bloom.insert("example");
```

### Checking for Membership
```rust
if bloom.contains("example") {
    println!("Item might be present");
} else {
    println!("Item is definitely not present");
}
```

### Resetting the Filter
```rust
bloom.reset();
```

## License
MIT

