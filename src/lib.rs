//! # Spork
//! A cross-platform module for measuring CPU and memory usage for processes, threads, and children threads.
//! This module currently supports Linux, Windows, and OS X.
//!
//! A few notes first.
//!
//! ## Windows
//! * As of right now, Windows support for child process information is currently unimplemented until a solution is found.
//! * Also, when polling for `StatType::Thread` the memory usage will be 0. See Spork struct documenation for details
//!
//! ## OSX
//! * When polling for `StatType::Thread`, memory usage reported will be different than Linux, See Spork struct documenation for details
//!
//! ## Linux
//! * No abnormalitites to note
//!
//! ## Basic Usage
//!
//! ```
//! extern crate spork;
//!
//! use spork::{
//!   Spork,
//!   SporkError,
//!   SporkErrorKind,
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

#[cfg(target_pointer_width = "32")]
pub type CLong = i32;

#[cfg(target_pointer_width = "64")]
pub type CLong = i64;

#[cfg(target_os = "macos")]
extern crate mach;

extern crate chrono;
extern crate kernel32;
extern crate libc;
extern crate psapi;
extern crate sys_info;
extern crate thread_id;
extern crate winapi;

mod utils;

use utils::History;

use std::io::Error as IoError;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod posix;

#[cfg(target_os = "macos")]
mod darwin;

/// The kind of SporkError
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SporkErrorKind {
    InvalidStatType,
    Unimplemented,
    Unknown,
}

/// A Spork error struct capturing information about errors coming from Spork
/// if compiled with the feature `compile_unimplemented
/// certain functions will always return `Unimplemented` errors at runtime
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SporkError {
    /// Representation of the ErrorKind. "Invalid Stat Type", "Unknown Error', etc
    desc: &'static str,
    /// Details about the particular error
    details: String,
    /// The kind of spork Error
    kind: SporkErrorKind,
}

impl SporkError {
    /// Create a new SporkError instance
    pub fn new<T: Into<String>>(kind: SporkErrorKind, details: T) -> SporkError {
        let desc = match kind {
            SporkErrorKind::InvalidStatType => "Invalid Stat Type",
            SporkErrorKind::Unimplemented => "Unimplemented",
            SporkErrorKind::Unknown => "Unknown Error",
        };

        SporkError {
            desc: desc,
            details: details.into(),
            kind: kind,
        }
    }

    /// Read the error's details
    pub fn details(&self) -> &str {
        &self.details
    }

    /// Read the error's kind
    pub fn kind(&self) -> &SporkErrorKind {
        &self.kind
    }

    /// Read a formatted string consisting of error desc and details
    pub fn to_string(&self) -> String {
        format!("{}: {}", &self.desc, &self.details)
    }

    /// Create a new `Error` instance from a borrowed str.
    pub fn new_borrowed(kind: SporkErrorKind, details: &str) -> SporkError {
        SporkError::new(kind, details.to_owned())
    }

    /// Convinience function for creating an empty `Unimplemented` error.
    pub fn unimplemented() -> SporkError {
        SporkError::new(SporkErrorKind::Unimplemented, String::new())
    }
}

impl From<sys_info::Error> for SporkError {
    fn from(error: sys_info::Error) -> Self {
        match error {
            sys_info::Error::UnsupportedSystem => {
                SporkError::new(SporkErrorKind::Unimplemented, "Unsupported system.".to_owned())
            }
            sys_info::Error::ExecFailed(e) => SporkError::new(SporkErrorKind::Unknown, e.to_string()),
            sys_info::Error::IO(e) => SporkError::new(SporkErrorKind::Unknown, e.to_string()),
            sys_info::Error::Unknown => {
                SporkError::new(SporkErrorKind::Unknown, "Sys_info encountered an unknown error.")
            }
        }
    }
}

impl From<IoError> for SporkError {
    fn from(error: IoError) -> Self {
        SporkError::new(SporkErrorKind::Unknown, format!("{}", error))
    }
}

/// An enum describing how to scope the CPU and memory data. `Process` reads CPU and memory usage across the entire process
/// and can be used with `stats_with_cpus`, `Children` reads CPU and memory for child threads of the calling thread and can
/// also be used with `stats_with_cpus`, and `Thread` reads CPU and memory for the calling thread only. On Linux or OS X (POSIX)
/// see [getrusage](http://man7.org/linux/man-pages/man2/getrusage.2.html) and [clock_gettime](http://man7.org/linux/man-pages/man2/clock_gettime.2.html)
/// for more information, and on Windows see
/// [GetProcessMemoryInfo](https://msdn.microsoft.com/en-us/library/windows/desktop/ms683219(v=vs.85).aspx) and
/// [GetProcessTimes](https://msdn.microsoft.com/en-us/library/windows/desktop/ms683223(v=vs.85).aspx).
/// For more information about OSX Thread usage stats see:
/// [TaskBasicInfo](http://web.mit.edu/darwin/src/modules/xnu/osfmk/man/task_basic_info.html)
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatType {
    /// Read usage across the entire process.
    Process,
    /// Read usage for the calling thread.
    Thread,
    /// Read usage across all child threads of the calling thread. This can mean different things on different platforms,
    /// and usually doesn't do what you want it to do. Use with caution.
    Children,
}

/// The current system's platform, such as `Linux`, `Windows`, etc.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

/// A struct holding CPU and memory usage information.
#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    /// Time at which the stats were polled, in milliseconds since epoch.
    pub polled: i64,
    /// Duration over which CPU usage was calculated, in milliseconds.
    pub duration: u64,
    /// Total CPU time spent on application.
    pub cpu_time: f64,
    /// Average CPU load (percentage) since the last poll.
    pub cpu: f64,
    /// Total working set size, in bytes. This can mean different things depending on the `StatType` used.
    pub memory: u64,
    /// Process uptime, in ms.
    pub uptime: u64,
    /// The type of statistic.
    pub kind: StatType,
    /// The number of CPU cores considered when measuring the CPU usage.
    pub cores: usize,
}

impl Stats {
    pub fn new_empty(kind: StatType) -> Stats {
        Stats {
            kind: kind,
            polled: 0,
            duration: 0,
            cpu_time: 0_f64,
            cpu: 0_f64,
            memory: 0,
            uptime: 0,
            cores: 1,
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
/// ## OSX
/// As noted above the memory statistic on OSX is slightly different than in Unix. Memory for OSX
/// returns the number of resident pages for the task
///
/// ## Windows
/// As noted above, the memory stat for windows returns 0 on StatType::Thread. This is due to (AFAIK) no way to get memory usage for a thread.
/// Any information about this is gracious accepted
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
    started: i64,
}

impl Spork {
    /// Create a new `Spork` instance.
    pub fn new() -> Result<Spork, SporkError> {
        Ok(Spork {
            history: History::default(),
            platform: utils::get_platform()?,
            clock: utils::get_cpu_speed()?,
            cpus: utils::get_num_cores()?,
            started: utils::now_ms(),
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
    #[cfg(target_os = "linux")]
    pub fn stats(&self, kind: StatType) -> Result<Stats, SporkError> {
        let now = utils::now_ms();
        let duration = utils::calc_duration(&kind, &self.history, self.started, now);

        let usage = posix::get_stats(&kind)?;
        let cpu_time = posix::get_cpu_time(&usage);
        let cpu_percent = utils::calc_cpu_percent(&self.history, &kind, cpu_time, duration);

        let stats = Stats {
            kind: kind.clone(),
            polled: now,
            duration: duration,
            cpu_time: cpu_time,
            cpu: cpu_percent,
            memory: (usage.ru_maxrss as u64) * 1000,
            uptime: utils::safe_unsigned_sub(now, self.started),
            cores: 1,
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
    #[cfg(target_os = "macos")]
    pub fn stats(&self, kind: StatType) -> Result<Stats, SporkError> {
        let now = utils::now_ms();
        let duration = utils::calc_duration(&kind, &self.history, self.started, now);

        let usage = darwin::get_stats(&kind)?;
        let cpu_time = darwin::get_cpu_time(&usage);
        let cpu_percent = utils::calc_cpu_percent(&self.history, &kind, cpu_time, duration);

        let stats = Stats {
            kind: kind.clone(),
            polled: now,
            duration: duration,
            cpu_time: cpu_time,
            cpu: cpu_percent,
            memory: (usage.ru_maxrss as u64) * 1000,
            uptime: utils::safe_unsigned_sub(now, self.started),
            cores: 1,
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
    #[cfg(target_os = "linux")]
    pub fn stats_with_cpus(&self, kind: StatType, cores: Option<usize>) -> Result<Stats, SporkError> {
        let cores = match cores {
            Some(c) => c,
            None => self.cpus,
        };

        if cores > self.cpus {
            return Err(SporkError::new_borrowed(SporkErrorKind::Unknown, "Invalid CPU count."));
        }

        let now = utils::now_ms();
        let duration = utils::calc_duration(&kind, &self.history, self.started, now);

        let usage = posix::get_stats(&kind)?;
        let cpu_time = posix::get_cpu_time(&usage);
        let cpu_percent = utils::calc_cpu_percent(&self.history, &kind, cpu_time, duration);

        let stats = Stats {
            kind: kind.clone(),
            polled: now,
            duration: duration,
            cpu_time: cpu_time,
            cpu: cpu_percent,
            memory: (usage.ru_maxrss as u64) * 1000,
            uptime: utils::safe_unsigned_sub(now, self.started),
            cores: cores,
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
    #[cfg(target_os = "macos")]
    pub fn stats_with_cpus(&self, kind: StatType, cores: Option<usize>) -> Result<Stats, SporkError> {
        let cores = match cores {
            Some(c) => c,
            None => self.cpus,
        };

        if cores > self.cpus {
            return Err(SporkError::new_borrowed(SporkErrorKind::Unknown, "Invalid CPU count."));
        }

        let now = utils::now_ms();
        let duration = utils::calc_duration(&kind, &self.history, self.started, now);

        let usage = darwin::get_stats(&kind)?;
        let cpu_time = darwin::get_cpu_time(&usage);
        let cpu_percent = utils::calc_cpu_percent(&self.history, &kind, cpu_time, duration);

        let stats = Stats {
            kind: kind.clone(),
            polled: now,
            duration: duration,
            cpu_time: cpu_time,
            cpu: cpu_percent,
            memory: (usage.ru_maxrss as u64) * 1000,
            uptime: utils::safe_unsigned_sub(now, self.started),
            cores: cores,
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
    pub fn stats(&self, kind: StatType) -> Result<Stats, SporkError> {
        let now = utils::now_ms();
        let duration = utils::calc_duration(&kind, &self.history, self.started, now);

        let cpu_times = windows::get_cpu_times(&kind)?;
        let cpu_time = windows::combine_cpu_times(&cpu_times);
        let mem = windows::get_mem_stats(&kind)?;

        let cpu_percent = utils::calc_cpu_percent(&self.history, &kind, cpu_time, duration);

        let stats = Stats {
            kind: kind.clone(),
            polled: now,
            duration: duration,
            cpu_time: cpu_time,
            cpu: cpu_percent,
            memory: (mem.PeakWorkingSetSize as u64) / 1024,
            uptime: utils::safe_unsigned_sub(now, self.started),
            cores: 1,
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
    #[cfg(windows)]
    pub fn stats_with_cpus(&self, kind: StatType, cores: Option<usize>) -> Result<Stats, SporkError> {
        let cores = match cores {
            Some(c) => c,
            None => self.cpus,
        };

        if cores > self.cpus {
            return Err(SporkError::new_borrowed(SporkErrorKind::Unknown, "Invalid CPU count."));
        }
        let now = utils::now_ms();
        let duration = utils::calc_duration(&kind, &self.history, self.started, now);

        let cpu_times = windows::get_cpu_times(&kind)?;
        let cpu_time = windows::combine_cpu_times(&cpu_times);
        let mem = windows::get_mem_stats(&kind)?;

        let cpu_percent = utils::calc_cpu_percent(&self.history, &kind, cpu_time, duration);

        let stats = Stats {
            kind: kind.clone(),
            polled: now,
            duration: duration,
            cpu_time: cpu_time,
            cpu: cpu_percent,
            memory: (mem.PeakWorkingSetSize as u64) / 1024,
            uptime: utils::safe_unsigned_sub(now, self.started),
            cores: cores,
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
    #[cfg(all(feature = "compile_unimplemented", not(any(unix, windows, target_os = "macos"))))]
    pub fn stats(&self, kind: StatType) -> Result<Stats, SporkError> {
        Err(SporkError::unimplemented())
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
    #[cfg(all(feature = "compile_unimplemented", not(any(unix, windows, target_os = "macos"))))]
    pub fn stats_with_cpus(&self, kind: StatType, cores: Option<usize>) -> Result<Stats, SporkError> {
        Err(SporkError::unimplemented())
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

    use super::*;

    use std::io;

    #[test]
    fn should_create_invalid_stat_errors() {
        let msg = "Foo";
        let error = SporkError::new_borrowed(SporkErrorKind::InvalidStatType, msg);
        match *error.kind() {
            SporkErrorKind::InvalidStatType => {
                assert_eq!(error.details(), msg);
                assert_eq!(error.to_string(), format!("Invalid Stat Type: {}", msg));
            }
            _ => panic!("Invalid eror enum {:?}! Expected InvalidStatType", error),
        };
    }

    #[test]
    fn should_create_unimplemented_errors() {
        let msg = "Bar";
        let error = SporkError::new_borrowed(SporkErrorKind::Unimplemented, msg);
        match *error.kind() {
            SporkErrorKind::Unimplemented => {
                assert_eq!(error.details(), msg);
                assert_eq!(error.to_string(), format!("Unimplemented: {}", msg));
            }
            _ => panic!("Invalid eror enum {:?}! Expected Unimplemented", error),
        };
    }

    #[test]
    fn should_create_unknown_errors() {
        let msg = "Baz";
        let error = SporkError::new_borrowed(SporkErrorKind::Unknown, msg);
        match *error.kind() {
            SporkErrorKind::Unknown => {
                assert_eq!(error.details(), msg);
                assert_eq!(error.to_string(), format!("Unknown Error: {}", msg));
            }
            _ => panic!("Invalid eror enum {:?}! Expected Unknown", error),
        };
    }

    #[test]
    fn should_shortcut_create_unimplemented_errors() {
        let error = SporkError::unimplemented();
        match *error.kind() {
            SporkErrorKind::Unimplemented => {
                assert_eq!(error.details(), "");
            }
            _ => panic!("Invalid unimplemented error! {:?}. Expected Unimplemented", error),
        };
    }

    #[test]
    fn should_create_error_from_new_borrowed() {
        let error = SporkError::new_borrowed(SporkErrorKind::Unimplemented, "Foo");
        match *error.kind() {
            SporkErrorKind::Unimplemented => {
                assert_eq!(error.details(), "Foo");
            }
            _ => panic!("Invalid unimplemented error! {:?}. Expected Unimplemented", error),
        };
    }

    #[test]
    fn should_create_error_from_sys_info_error() {
        let err = sys_info::Error::UnsupportedSystem;
        let error: SporkError = err.into();
        match *error.kind() {
            SporkErrorKind::Unimplemented => {
                assert_eq!(error.details(), "Unsupported system.");
            }
            _ => panic!("Invalid unimplemented error! {:?}. Expected Unimplemented", error),
        };
    }

    #[test]
    fn should_create_error_from_io_error() {
        let err = io::Error::last_os_error();
        let error: SporkError = err.into();
        match *error.kind() {
            SporkErrorKind::Unknown => {
                assert!(true);
            }
            _ => panic!("Invalid unimplemented error! {:?}. Expected Unimplemented", error),
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
        assert!(spork.num_cores() > 0);
    }

    #[test]
    fn should_get_cpu_speed() {
        let spork = Spork::new().unwrap();
        assert!(spork.clock_speed() > 0);
    }

    #[test]
    #[cfg(target_os = "linux")]
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
    #[cfg(target_os = "macos")]
    fn should_get_macos_platform() {
        let spork = Spork::new().unwrap();
        assert_eq!(spork.platform(), Platform::MacOS);
    }

    #[test]
    #[cfg(all(feature = "compile_unimplemented", not(any(unix, windows, target_os = "macos"))))]
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
    #[cfg(windows)]
    fn should_get_windows_stats_with_cpus() {}

    #[test]
    #[cfg(target_os = "macos")]
    fn should_get_macos_stats_with_cpus() {}

    #[test]
    #[cfg(all(feature = "compile_unimplemented", not(any(unix, windows, target_os = "macos"))))]
    fn should_err_on_unimplemented_stats() {
        let spork = Spork::new().unwrap();
        if let Ok(_) = spork.stats(StatType::Thread) {
            panic!("Stat error not thrown on unimplemented platform!");
        }
    }

    #[test]
    #[cfg(all(feature = "compile_unimplemented", not(any(unix, windows, target_os = "macos"))))]
    fn should_err_on_unimplemented_stats_with_cpus() {
        let spork = Spork::new().unwrap();
        if let Ok(_) = spork.stats_with_cpus(StatType::Thread, None) {
            panic!("Stat error not thrown on unimplemented platform!");
        }
    }
}
