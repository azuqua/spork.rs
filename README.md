Spork
=====

CPU and memory usage for threads and processes.

[![Build Status](https://travis-ci.org/azuqua/spork.rs.svg?branch=master)](https://travis-ci.org/azuqua/spork.rs)
[![Coverage Status](https://coveralls.io/repos/github/azuqua/spork.rs/badge.svg)](https://coveralls.io/github/azuqua/spork.rs)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[Documentation](https://azuqua.github.io/spork.rs)

# Install

With [Cargo Edit](https://github.com/killercup/cargo-edit):

```
cargo add spork
```

Or add to your `Cargo.toml`

```
spork = "0.1"
```

# Status

Currently POSIX compliant platforms are developed and tested, but Windows support remains a WIP.

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

println!("Thread CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  t_stats.cpu, t_stats.memory, t_stats.cores, t_stats.kind, t_stats.polled);

// get process stats across all CPU cores
let p_stats = spork.stats_with_cpus(StatType::Process, None).unwrap();

// get process stats across only 2 cores
let p_stats = spork.stats_with_cpus(StatType::Process, Some(2)).unwrap();

// get stats for child threads of the calling thread across all CPU cores
let c_stats = spork.stats_with_cpus(StatType::Children, None).unwrap();
```

# Unsupported Platforms

This module supports POSIX compliant platforms (Linux, OS X, etc) and Windows (soon). If you'd like to use this on an unsupported platform, or one on which you might expect compatibility issues, there are two options available for testing and usage. If you'd prefer to catch any compatibility issues at compile-time just download this library and try to build it. If it builds it should work, but it's still a good idea to run the test suite before trying it in production. 

If you'd prefer to handle compatibility errors at runtime add the `compile_unimplemented` feature to your Cargo.toml for Spork. Instead of introducing compiler errors this will compile mock functions which always return `Unimplemented` errors in place of any missing platform-specific ones.

# Tests

```
cargo test --lib
```

