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
/// see [getrusage](http://man7.org/linux/man-pages/man2/getrusage.2.html) for more information, and on windows see 
/// [GetProcessMemoryInfo](https://msdn.microsoft.com/en-us/library/windows/desktop/ms683219(v=vs.85).aspx) and 
/// [GetProcessTimes](https://msdn.microsoft.com/en-us/library/windows/desktop/ms683223(v=vs.85).aspx).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatType {
  /// Read usage across the entire process.
  Process,
  /// Read usage across all child threads of the calling thread. This can mean different things on different platforms.
  Children,
  /// Read usage for the calling thread.
  Thread
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
#[derive(Clone, Debug)]
pub struct Stats {
  /// Time at which the stats were polled, in ms since epoch.
  pub polled: u64,
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
    let stats = posix::get_stats(&kind);

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
  #[cfg(any(unix, target_os="macos"))]
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

}

// ---------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn should_create_invalid_stat_errors() {
    let msg = "Foo";
    let error = Error::new(ErrorKind::InvalidStatType, msg.to_owned());
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
    let error = Error::new(ErrorKind::Unimplemented, msg.to_owned());
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
    let error = Error::new(ErrorKind::Unknown, msg.to_owned());
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
  #[cfg(unix)]
  fn should_get_linux_stats() {

  }

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
  fn should_get_linux_stats_with_cpus() {

  }

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
