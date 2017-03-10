//! # Spork
//! 
//! A cross-platform module for measuring CPU and memory usage for processes, threads, and children threads. 
//! This module currently supports Linux, Windows, and OS X.
//!
//! ## Basic Usage 
//! 
//! ```
//! extern crate spork;
//!
//! use spork::{
//!   Spork,
//!   Error,
//!   ErrorKind,
//!   Platform,
//!   StatType,
//!   Stats
//! };
//!
//! let spork = match Spork::new() {
//!   Ok(s) => s,
//!   Err(e) => panic!("Error creating spork client! {:?}", e)
//! };
//!
//! println!("Using platform {:?}", spork.platform());
//! println!("CPU cores: {:?}x @ {:?} Hz", spork.num_cores(), spork.clock_speed());
//! 
//! // get process stats 
//! let p_stats = match spork.stats(StatType::Process) {
//!   Ok(s) => s,
//!   Err(e) => panic!("Error polling process stats! {:?}", e)
//! };
//! println!("Process stats: {:?}", p_stats);
//!
//! // get thread stats 
//! let t_stats = match spork.stats(StatType::Thread) {
//!   Ok(s) => s,
//!   Err(e) => panic!("Error polling thread stats! {:?}", e)
//! };
//! println!("Thread stats: {:?}", t_stats);
//! ```


extern crate chrono;
extern crate sys_info;
extern crate libc;
extern crate winapi;
extern crate kernel32;
extern crate psapi;
extern crate thread_id;

mod utils;

use utils::History;

use std::io::Error as IoError;

#[cfg(windows)]
mod windows;

#[cfg(any(unix, target_os="macos"))]
mod posix;

 
/// An error type describing the possible error cases for this module. If compiled with the feature `compile_unimplemented`
/// certain functions will always return `Unimplemented` errors at runtime. See the feature notes for more information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
  InvalidStatType { desc: &'static str, details: String },
  Unimplemented { desc: &'static str, details: String },
  Unknown { desc: &'static str, details: String }
}

/// The kind of error being created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
  InvalidStatType,
  Unimplemented,
  Unknown
}


impl From<sys_info::Error> for Error {
  fn from(error: sys_info::Error) -> Self {
    match error {
      sys_info::Error::UnsupportedSystem => {
        Error::new(ErrorKind::Unimplemented, "Unsupported system.".to_owned())
      },
      sys_info::Error::ExecFailed(s) => {
        Error::new(ErrorKind::Unknown, s)
      }
    }
  }
}

impl From<IoError> for Error {
  fn from(error: IoError) -> Self {
    Error::new(ErrorKind::Unknown, format!("{}", error))
  }
}

impl Error {
  
  /// Create a new `Error` instance.
  pub fn new(kind: ErrorKind, details: String) -> Error {
    match kind {
      ErrorKind::InvalidStatType => {
        Error::InvalidStatType {
          desc: "Invalid stat type",
          details: details
        }
      },
      ErrorKind::Unimplemented => {
        Error::Unimplemented {
          desc: "Unimplemented",
          details: details
        }
      },
      ErrorKind::Unknown => {
        Error::Unknown {
          desc: "Unknown",
          details: details
        }
      }
    }
  }

  /// Create a new `Error` instance from a borrowed str.
  pub fn new_borrowed(kind: ErrorKind, details: &str) -> Error {
    Error::new(kind, details.to_owned())
  }

  /// Shortcut for creating an empty `Unimplemented` error.
  pub fn unimplemented() -> Error {
    Error::new(ErrorKind::Unimplemented, String::new())
  }

  /// Read the error's details.
  pub fn inner(&self) -> &str {
    match *self {
      Error::InvalidStatType { desc: _, details: ref details } => details,
      Error::Unimplemented { desc: _, details: ref details } => details,
      Error::Unknown { desc: _, details: ref details } => details
    }
  }

  /// Read a copy of the error's details.
  pub fn inner_owned(&self) -> String {
    self.inner().to_owned()
  }

}


/// An enum describing how to scope the CPU and memory data. `Process` reads CPU and memory usage across the entire process
/// and can be used with `stats_with_cpus`, `Children` reads CPU and memory for child threads of the calling thread and can 
/// also be used with `stats_with_cpus`, and `Thread` reads CPU and memory for the calling thread only. On Linux or OS X (POSIX) 
/// see [getrusage](http://man7.org/linux/man-pages/man2/getrusage.2.html) and [clock_gettime](http://man7.org/linux/man-pages/man2/clock_gettime.2.html) 
/// for more information, and on Windows see 
/// [GetProcessMemoryInfo](https://msdn.microsoft.com/en-us/library/windows/desktop/ms683219(v=vs.85).aspx) and 
/// [GetProcessTimes](https://msdn.microsoft.com/en-us/library/windows/desktop/ms683223(v=vs.85).aspx).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatType {
  /// Read usage across the entire process.
  Process,
  /// Read usage for the calling thread.
  Thread,
  /// Read usage across all child threads of the calling thread. This can mean different things on different platforms,
  /// and usually doesn't do what you want it to do. Use with caution, here be dragons.
  Children
}

/// The current system's platform, such as `Linux`, `Windows`, etc.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Platform {
  Linux,
  MacOS,
  Windows,
  Unknown
}

/// A struct holding CPU and memory usage information.
#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
  /// Time at which the stats were polled, in milliseconds since epoch.
  pub polled: i64,
  /// Duration over which CPU usage was calculated, in milliseconds.
  pub duration: u64,
  /// Average CPU load (percentage) since the last poll.
  pub cpu: f64,
  /// Total working set size, in bytes. This can mean different things depending on the `StatType` used.
  pub memory: u64,
  /// Process uptime, in ms. 
  pub uptime: u64,
  /// The type of statistic.
  pub kind: StatType,
  /// The number of CPU cores considered when measuring the CPU usage.
  pub cores: usize
}

impl Stats {

  pub fn new_empty(kind: StatType) -> Stats {
    Stats {
      kind: kind,
      polled: 0,
      duration: 0,
      cpu: 0_f64,
      memory: 0,
      uptime: 0,
      cores: 1
    }
  }

}


/// A struct to monitor CPU and memory usage. 
///
/// ### Important Notes: 
/// 
/// The `stats` and `stats_with_cpus` functions measure CPU usage by the time between calls (in ms), separated by 
/// the provided `StatType`. For example, a single thread calling `stat` with the `StatType::Thread` option in a loop 
/// every second will correctly measure usage across the previous 1 second interval for only that thread. However, if 
/// for example two threads running in loops with different delays both try to read `stats` with `StatType::Process` 
/// then the interval over which the CPU load is calculated will be the delta between whenever the function was last 
/// called in _either_ thread. Make sure your program accounts for this by using the correct `StatType` such that 
/// multiple threads do not interfere with each other's intended sample rates. Using `stat` and `stat_with_cpus` with
/// `StatType::Thread` across multiple concurrent threads will correctly measure the results for each thread individually.
///
/// TLDR: Be careful using `stat` and `stat_with_cpus` with `StatType::Process` across multiple threads running concurrently.
///
#[derive(Clone, Debug)]
pub struct Spork {
  history: History,
  platform: Platform,
  clock: u64,
  cpus: usize,
  // TODO use process uptime
  started: i64
}

impl Spork {

  /// Create a new `Spork` instance.
  pub fn new() -> Result<Spork, Error> {  
    Ok(Spork {
      history: History::default(),
      platform: try!(utils::get_platform()),
      clock: try!(utils::get_cpu_speed()),
      cpus: try!(utils::get_num_cores()),
      started: utils::now_ms()
    })
  }
  
  /// Get CPU and memory statistics in a `Stats` instance for the provided `StatType` assuming usage across only 1 CPU core.
  /// 
  /// ```
  /// let spork = Spork::new().unwrap();
  /// let stats = spork.stats(StatType::Thread).unwrap();
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  /// ```
  #[cfg(any(unix, target_os="macos"))]
  pub fn stats(&self, kind: StatType) -> Result<Stats, Error> {
    let now = utils::now_ms();
    let duration = utils::calc_duration(&kind, &self.history, self.started, now);

    let usage = try!(posix::get_stats(&kind));
    let cpu = posix::get_cpu_percent(self.clock, duration, &usage);

    let stats = Stats {
      kind: kind.clone(),
      polled: now,
      duration: duration,
      cpu: cpu,
      memory: (usage.ru_maxrss as u64) * 1000,
      uptime: utils::safe_unsigned_sub(now, self.started),
      cores: 1
    };

    self.history.set_last(&kind, stats.clone());
    Ok(stats)
  }

  /// Get CPU and memory statistics in a `Stats` instance for the provided `StatType` assuming usage across `count` CPU core(s).
  /// If `None` is provided then all available CPU cores will be considered, where applicable.
  ///
  /// ```
  /// let spork = Spork::new().unwrap();
  /// // read stats across all available CPU cores
  /// let stats = spork.stats_with_cpus(StatType::Thread, None).unwrap();
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  ///
  /// // read stats considering only 2 CPU cores
  /// let stats = spork.stats_with_cpus(StatType::Thread, Some(2));
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  /// ```
  #[cfg(any(unix, target_os="macos"))]
  pub fn stats_with_cpus(&self, kind: StatType, cores: Option<usize>) -> Result<Stats, Error> {
    let cores = match cores {
      Some(c) => c,
      None => self.cpus
    };

    if cores > self.cpus {
      return Err(Error::new_borrowed(ErrorKind::Unknown, "Invalid CPU count."));
    }

    let freq = utils::scale_freq_by_cores(self.clock, cores);
    let now = utils::now_ms();
    let duration = utils::calc_duration(&kind, &self.history, self.started, now);

    let usage = try!(posix::get_stats(&kind));
    let cpu = posix::get_cpu_percent(freq, duration, &usage);

    let stats = Stats {
      kind: kind.clone(),
      polled: now,
      duration: duration,
      cpu: cpu,
      memory: (usage.ru_maxrss as u64) * 1000,
      uptime: utils::safe_unsigned_sub(now, self.started),
      cores: cores
    };

    self.history.set_last(&kind, stats.clone());
    Ok(stats)
  }

  /// Get CPU and memory statistics in a `Stats` instance for the provided `StatType` assuming usage across only 1 CPU core.
  /// 
  /// ```
  /// let spork = Spork::new().unwrap();
  /// let stats = spork.stats(StatType::Thread).unwrap();
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  /// ```
  #[cfg(windows)]
  pub fn stats(&self, kind: StatType) -> Result<Stats, Error> {
    let cpu_times = try!(windows::get_cpu_times(&kind));
    let mem = try!(windows::get_mem_stats(&kind));

    unimplemented!();
  }

  /// Get CPU and memory statistics in a `Stats` instance for the provided `StatType` assuming usage across `count` CPU core(s).
  /// If `None` is provided then all available CPU cores will be considered, where applicable.
  ///
  /// ```
  /// let spork = Spork::new().unwrap();
  /// // read stats across all available CPU cores
  /// let stats = spork.stats_with_cpus(StatType::Thread, None).unwrap();
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  ///
  /// // read stats considering only 2 CPU cores
  /// let stats = spork.stats_with_cpus(StatType::Thread, Some(2));
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  /// ```
  #[cfg(windows)]
  pub fn stats_with_cpus(&self, kind: StatType, cores: Option<usize>) -> Result<Stats, Error> {
    unimplemented!();
  }

  /// Get CPU and memory statistics in a `Stats` instance for the provided `StatType` assuming usage across only 1 CPU core.
  /// 
  /// ```
  /// let spork = Spork::new().unwrap();
  /// let stats = spork.stats(StatType::Thread).unwrap();
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  /// ```
  #[cfg(all(feature="compile_unimplemented", not(any(unix, windows, target_os="macos"))))]
  pub fn stats(&self, kind:StatType) -> Result<Stats, Error> {
    Err(Error::unimplemented()) 
  }

  /// Get CPU and memory statistics in a `Stats` instance for the provided `StatType` assuming usage across `count` CPU core(s).
  /// If `None` is provided then all available CPU cores will be considered, where applicable.
  ///
  /// ```
  /// let spork = Spork::new().unwrap();
  /// // read stats across all available CPU cores
  /// let stats = spork.stats_with_cpus(StatType::Thread, None).unwrap();
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  ///
  /// // read stats considering only 2 CPU cores
  /// let stats = spork.stats_with_cpus(StatType::Thread, Some(2));
  ///
  /// println!("CPU: {}%, Memory: {} bytes, Cores: {}, Type: {}, Polled at: {}", 
  ///   stats.cpu, stats.memory, stats.cores, stats.kind, stats.polled);
  /// ```
  #[cfg(all(feature="compile_unimplemented", not(any(unix, windows, target_os="macos"))))]
  pub fn stats_with_cpus(&self, kind: StatType, cores: Option<usize>) -> Result<Stats, Error> {
    unimplemented!();
  }

  /// Get the system type.
  pub fn platform(&self) -> Platform {
    self.platform.clone()
  }

  /// Get the CPU clock speed, in Hz.
  pub fn clock_speed(&self) -> u64 {
    self.clock
  }

  /// Get the number of CPU cores for your system.
  pub fn num_cores(&self) -> usize {
    self.cpus
  }

  /// Clear the stats history for the process or calling thread. This library works by tracking the timestamp of the last stats poll, per thread, such that polls from different threads do not interfere with each other.
  /// However, the downside to this approach is that some extra data has to stick around. This function will delete the timestamp of the previous poll for the process or calling thread, and if the same thread
  /// decides to call `stats` or `stats_with_cpus` again it will use the process' uptime as the duration over which to calculate CPU usage for the next call.
  /// If a `Spork` instance is shared among multiple threads with short lifespans then it's a good idea to call this when those threads exit.
  pub fn drop_history(&self, kind: StatType) -> Option<Stats> {
    self.history.clear_last(&kind)
  }

  /// Read a copy of the most recently polled stats for `kind`.
  pub fn read_history(&self, kind: StatType) -> Option<Stats> {
    self.history.get_last(&kind)
  }

}

// ---------------------

#[cfg(test)]
mod tests {
  extern crate rand;
  use self::rand::distributions::{IndependentSample, Range};

  use super::*;

  use std::thread;
  use std::time;

  macro_rules! sleep_ms(
    ($($arg:tt)*) => { {
      thread::sleep(time::Duration::from_millis($($arg)*))
    } } 
  );

  fn fib(n: u64) -> u64 {
    if n > 2 {
      fib(n - 1) + fib(n - 2) 
    } else {
      1
    }
  }

  // not ideal, but it's just for tests
  fn rand_in_range(l: u64, r: u64) -> u64 {
    let between = Range::new(l, r);
    let mut rng = rand::thread_rng();
    between.ind_sample(&mut rng)
  }

  #[test]
  fn should_create_invalid_stat_errors() {
    let msg = "Foo";
    let error = Error::new_borrowed(ErrorKind::InvalidStatType, msg);
    match error {
      Error::InvalidStatType { details: _, desc: _ } => {
        assert_eq!(error.inner(), msg);
      },
      _ => panic!("Invalid eror enum {:?}! Expected InvalidStatType", error)
    };
  }

  #[test]
  fn should_create_unimplemented_errors() {
    let msg = "Bar";
    let error = Error::new_borrowed(ErrorKind::Unimplemented, msg);
    match error {
      Error::Unimplemented { details: _, desc: _ } => {
        assert_eq!(error.inner(), msg);
      },
      _ => panic!("Invalid eror enum {:?}! Expected Unimplemented", error)
    };
  }

  #[test]
  fn should_create_uknown_errors() {
    let msg = "Baz";
    let error = Error::new_borrowed(ErrorKind::Unknown, msg);
    match error {
      Error::Unknown { details: _, desc: _ } => {
        assert_eq!(error.inner(), msg);
      },
      _ => panic!("Invalid eror enum {:?}! Expected Unknown", error)
    };
  }

  #[test]
  fn should_shortcut_create_unimplemented_errors() {
    let error = Error::unimplemented();
    match error {
      Error::Unimplemented { details: _, desc: _ } => {
        assert_eq!(error.inner(), "");
      },
      _ => panic!("Invalid unimplemented error! {:?}. Expected Unimplemented", error)
    };
  }

  #[test]
  fn should_create_spork() {
    if let Err(e) = Spork::new() {
      panic!("Error creating spork instance {:?}", e);
    }
  }

  #[test]
  fn should_get_cpu_cores() {
    let spork = Spork::new().unwrap();
    // FIXME make this smarter
    assert!(spork.num_cores() > 0);
  }

  #[test]
  fn should_get_cpu_speed() {
    let spork = Spork::new().unwrap();
    // FIXME make this smarter
    assert!(spork.clock_speed() > 0);
  }

  #[test]
  #[cfg(unix)]
  fn should_get_linux_platform() {
    let spork = Spork::new().unwrap();
    assert_eq!(spork.platform(), Platform::Linux);
  }

  #[test]
  #[cfg(windows)]
  fn should_get_windows_platform() {
    let spork = Spork::new().unwrap();
    assert_eq!(spork.platform(), Platform::Windows);
  }

  #[test]
  #[cfg(target_os="macos")]
  fn should_get_macos_platform() {
    let spork = Spork::new().unwrap();
    assert_eq!(spork.platform(), Platform::MacOS);
  }

  #[test]
  #[cfg(all(feature="compile_unimplemented", not(any(unix, windows, target_os="macos"))))]
  fn should_get_unknown_platform() {
    let spork = Spork::new().unwrap();
    assert_eq!(spork.platform(), Platform::Unknown);
  }

  #[test]
  fn should_read_spork_history() {
    let spork = Spork::new().unwrap();
    assert_eq!(spork.read_history(StatType::Process), None);
    assert_eq!(spork.read_history(StatType::Thread), None);
    assert_eq!(spork.read_history(StatType::Children), None);
  }

  #[test]
  fn should_clear_spork_history() {
    let spork = Spork::new().unwrap();
    assert_eq!(spork.drop_history(StatType::Process), None);
    assert_eq!(spork.drop_history(StatType::Thread), None);
    assert_eq!(spork.drop_history(StatType::Children), None);
  }

  #[test]
  #[cfg(unix)]
  fn should_get_linux_process_stats_fib_25() {
    // intentionally introduce some delays to simulate some weird contention for the clocks among
    // testing threads in order to hopefully draw out any bugs scoping the results between threads
    let wait = rand_in_range(100, 400);
    let expected_cpu = 10_f64;

    let before = utils::now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(25);

    let stats = match spork.stats(StatType::Process) {
      Ok(s) => s,
      Err(e) => panic!("Stats error {:?}", e)
    };
    let _final = utils::now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.memory > 0);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, 1);
    assert_eq!(stats.kind, StatType::Process);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
  }

  #[test]
  #[cfg(unix)]
  fn should_get_linux_thread_stats_fib_35() {
    let wait = rand_in_range(100, 400);
    let expected_cpu = 10_f64;

    let before = utils::now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(35);

    let stats = match spork.stats(StatType::Thread) {
      Ok(s) => s,
      Err(e) => panic!("Stats error {:?}", e)
    };
    let _final = utils::now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.memory > 0);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, 1);
    assert_eq!(stats.kind, StatType::Thread);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
  }

  // this is a huge pain to test, and results are spotty at best
  /*
  #[test]
  #[cfg(unix)]
  fn should_get_linux_children_stats_fib_25() {
    let wait = rand_in_range(100, 400);
    let before = utils::now_ms() as u64;
    let spork = Spork::new().unwrap();

    let jh = thread::spawn(move || {
      sleep_ms!(wait);
      fib(35);
    });
    let _ = jh.join();

    let stats = match spork.stats(StatType::Children) {
      Ok(s) => s,
      Err(e) => panic!("Stats error {:?}", e)
    };
    let _final = utils::now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > 10_f64);
    assert!(stats.memory > 0);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, 1);
    assert_eq!(stats.kind, StatType::Children);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
  }
  */

  #[test]
  #[cfg(windows)]
  fn should_get_windows_stats() {


  }

  #[test]
  #[cfg(target_os="macos")]
  fn should_get_macos_stats() {


  }

  #[test]
  #[cfg(unix)]
  fn should_get_linux_process_stats_with_cpus() {
    let wait = rand_in_range(100, 400);
    let expected_cpu = 5_f64;

    let before = utils::now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(25);

    let stats = match spork.stats_with_cpus(StatType::Process, Some(spork.num_cores())) {
      Ok(s) => s,
      Err(e) => panic!("Stats error {:?}", e)
    };
    let _final = utils::now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.memory > 0);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, spork.num_cores());
    assert_eq!(stats.kind, StatType::Process);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
  }

  #[test]
  #[cfg(unix)]
  fn should_get_linux_thread_stats_with_cpus() {
    let wait = rand_in_range(100, 400);
    let expected_cpu = 5_f64;

    let before = utils::now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(35);

    let stats = match spork.stats_with_cpus(StatType::Thread, Some(spork.num_cores())) {
      Ok(s) => s,
      Err(e) => panic!("Stats error {:?}", e)
    };
    let _final = utils::now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.memory > 0);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, spork.num_cores());
    assert_eq!(stats.kind, StatType::Thread);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
  }

  // see should_get_linux_children_stats_fib_25
  /*
  #[test]
  #[cfg(unix)]
  fn should_get_linux_children_stats_with_cpus() {
    let wait = rand_in_range(100, 400);
    let expected_cpu = 5_f64;

    let before = utils::now_ms() as u64;
    let spork = Spork::new().unwrap();

    let jh = thread::spawn(move || {
      sleep_ms!(wait);
      fib(35);
    });
    let _ = jh.join();

    let stats = match spork.stats_with_cpus(StatType::Children, Some(spork.num_cores())) {
      Ok(s) => s,
      Err(e) => panic!("Stats error {:?}", e)
    };
    let _final = utils::now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.memory > 0);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, spork.num_cores());
    assert_eq!(stats.kind, StatType::Children);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
  }
  */

  #[test]
  #[cfg(windows)]
  fn should_get_windows_stats_with_cpus() {


  }

  #[test]
  #[cfg(target_os="macos")]
  fn should_get_macos_stats_with_cpus() {


  }

  #[test]
  #[cfg(all(feature="compile_unimplemented", not(any(unix, windows, target_os="macos"))))]
  fn should_err_on_unimplemented_stats() {
    let spork = Spork::new().unwrap();
    if let Ok(_) = spork.stats(StatType::Thread) {
      panic!("Stat error not thrown on unimplemented platform!");
    }
  }

  #[test]
  #[cfg(all(feature="compile_unimplemented", not(any(unix, windows, target_os="macos"))))]
  fn should_err_on_unimplemented_stats_with_cpus() {
    let spork = Spork::new().unwrap();
    if let Ok(_) = spork.stats_with_cpus(StatType::Thread, None) {
      panic!("Stat error not thrown on unimplemented platform!");
    }
  }


}
