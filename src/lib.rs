//! Spork
//!
//!
//!
//!
//!
//!
//!

extern crate chrono;
extern crate sys_info;
extern crate libc;
extern crate winapi;
extern crate kernel32;
extern crate psapi;

mod utils;

use utils::History;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
mod linux;

#[cfg(target_os="macos")]
mod macos;
 
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
  InvalidStatType { desc: &'static str, details: String },
  Unimplemented { desc: &'static str, details: String },
  Unknown { desc: &'static str, details: String }
}

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


impl Error {
  
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

  pub fn unimplemented() -> Error {
    Error::new(ErrorKind::Unimplemented, String::new())
  }

}


/// 
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatType {
  Process,
  Children,
  Thread
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Platform {
  Linux,
  MacOS,
  Windows,
  Unknown
}

#[derive(Clone, Debug)]
pub struct Stats {
  /// Time at which the stats were polled, in ms since epoch.
  pub polled: u64,
  /// Average CPU load (percentage) since the last poll.
  pub cpu: f64,
  /// Total working set size, in bytes.
  pub memory: u64,
  /// Process uptime, in ms. 
  pub uptime: u64,
  /// The type of statistic.
  pub kind: StatType
}


/// 
#[derive(Debug)]
pub struct Spork {
  history: History,
  platform: Platform,
  clock: i64,
  // TODO use process uptime
  started: i64
}

impl Spork {

  pub fn new() -> Result<Spork, Error> {  
    Ok(Spork {
      platform: try!(utils::get_platform()),
      history: History::default(),
      clock: try!(utils::get_clock_ticks()),
      started: utils::now_ms()
    })
  }
  
  #[cfg(unix)]
  pub fn stats(&self, kind: StatType) -> Result<Stats, Error> {
    let stats = linux::get_stats(&kind);

    unimplemented!();
  }

  #[cfg(windows)]
  pub fn stats(&self, kind: StatType) -> Result<Stats, Error> {
    let cpu_times = try!(windows::get_cpu_times(&kind));
    let mem = try!(windows::get_mem_stats(&kind));

    unimplemented!();
  }

  #[cfg(target_os="macos")]
  pub fn stats(&self, kind: StatType) -> Result<Stats, Error> {
    unimplemented!();
  }

  #[cfg(all(feature="compile_unimplemented", not(any(unix, windows, target_os="macos"))))]
  pub fn stats(&self, kind:StatType) -> Result<Stats, Error> {
    Err(Error::unimplemented()) 
  }

  /// Get the system type.
  pub fn sys_type(&self) -> Platform {
    self.platform.clone()
  }

}


// ---------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn should_create_errors() {


  }

  #[test]
  fn should_create_pidusage() {

  }


  


}
