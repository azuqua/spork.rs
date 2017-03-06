Spork
=====

CPU and memory usage inspection for threads and processes.

[![Build Status](https://travis-ci.org/azuqua/spork.rs.svg?branch=master)](https://travis-ci.org/azuqua/spork.rs)
[![Coverage Status](https://coveralls.io/repos/github/azuqua/spork.rs/badge.svg)](https://coveralls.io/github/azuqua/spork.rs)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[Documentation](https://azuqua.github.io/spork.rs)

# Install

With [Cargo Edit](https://github.com/killercup/cargo-edit):

```
cargo add spork
```

Or by hand, add to your `Cargo.toml`

```
spork = "0.1"
```

# Usage

```rust
extern crate spork;

use spork::{
  Spork,
  Error,
  ErrorKind,
  Platform,
  StatType,
  Stats
};

let spork = match Spork::new() {
  Ok(s) => s,
  Err(e) => panic!("Error creating spork client! {:?}", e)
};

println!("Using platform {:?}", spork.platform());
println!("CPU cores: {:?}x @ {:?} Hz", spork.num_cores(), spork.clock_speed());

// get process stats 
let p_stats = match spork.stats(StatType::Process) {
  Ok(s) => s,
  Err(e) => panic!("Error polling process stats! {:?}", e)
};
println!("Process stats: {:?}", p_stats);

// get thread stats 
let t_stats = match spork.stats(StatType::Thread) {
  Ok(s) => s,
  Err(e) => panic!("Error polling thread stats! {:?}", e)
};
println!("Thread stats: {:?}", t_stats);
```

# Tests

```
cargo test
```

