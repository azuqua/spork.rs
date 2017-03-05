
use sys_info;
use chrono::UTC;

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuTime {
  pub sec: u64,
  pub usec: u64
}

#[derive(Clone, Debug)]
pub struct History {
  process: Option<Stats>,
  thread: Option<Stats>,
  children: Option<Stats>
}

impl Default for History {
  
  fn default() -> Self {
    History {
      process: None,
      thread: None,
      children: None
    }
  }

}

impl History {
  
  pub fn set_last(&mut self, kind: &StatType, poll: Stats) -> Option<Stats> {
    let last = self.get_last(kind);
    
     match *kind {
      StatType::Process => { self.process = Some(poll); },
      StatType::Thread => { self.thread = Some(poll); },
      StatType::Children => { self.children = Some(poll); }
    };

    last
  }

  pub fn get_last(&self, kind: &StatType) -> Option<Stats> {
    match *kind {
      StatType::Process => self.process.clone(),
      StatType::Thread => self.thread.clone(),
      StatType::Children => self.children.clone()
    }
  }

}

pub fn safe_unsigned_sub(lhs: i64, rhs: i64) -> u64 {
  (lhs - rhs).wrapping_abs() as u64
}

pub fn calc_duration(kind: &StatType, history: &History, started: u64, polled: u64) -> u64 {
  let last = match history.get_last(kind) {
    Some(stats) => stats.polled,
    None => started
  };

  last - polled
}

pub fn now_ms() -> i64 {
  let now = UTC::now();
  (now.timestamp() * 1000 + (now.timestamp_subsec_millis() as i64)) as i64
}

pub fn get_platform() -> Result<Platform, Error> {
  match try!(sys_info::os_type()).as_ref() {
    "Linux" => Ok(Platform::Linux),
    "Windows" => Ok(Platform::Windows),
    "Darwin" => Ok(Platform::MacOS),
    _ => Ok(Platform::Unknown)
  }
}

pub fn calc_cpu_percent(duration_ms: u64, hz: u64, cpu: &CpuTime) -> f64 {
  let cpu_time: f64 = (cpu.sec as f64) + (cpu.usec as f64 / 1000000_f64);
  let cpu_time_ms = cpu_time * 1000_f64;
  let cycles_ms = (hz as f64) * 1000_f64;
  let cycles_in_duration = (duration_ms as f64) *  cycles_ms;
  let used_cycles = cpu_time_ms * cycles_ms;
  
  100_f64 * (used_cycles / cycles_in_duration)
}

pub fn get_cpu_speed() -> Result<u64, Error> {
  match sys_info::cpu_speed() {
    Ok(s) => Ok(s * 1000),
    Err(e) => Err(Error::from(e))
  }
}

#[cfg(windows)]
pub use windows::get_clock_ticks;

#[cfg(unix)]
pub use linux::get_clock_ticks;

#[cfg(target_os="macos")]
pub use macos::get_clock_ticks;

#[cfg(all(feature="compile_unimplemented", not(any(unix, windows, target_os="macos"))))]
pub fn get_clock_ticks() -> Result<u64, Error> {
  Err(Error::unimplemented())
}

// ---------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn should_get_platform() {
    let platform = get_platform();
    assert_eq!(platform, Ok(Platform::Linux));
  }

  #[test]
  fn should_get_cpu_speed() {
    let speed = match get_cpu_speed() {
      Ok(s) => s,
      Err(e) => panic!("{:?}", e)
    };

    assert!(speed > 0);
  }

  #[test]
  fn should_calc_full_cpu() {
    let dur = 1000;
    let hz = 1000000;
    let time = CpuTime {
      sec: 1,
      usec: 0
    };
    let expected = 100_f64;

    assert_eq!(calc_cpu_percent(dur, hz, &time), expected);
  }

  #[test]
  fn should_calc_half_cpu() {
    let dur = 1000;
    let hz = 1000000;
    let time = CpuTime {
      sec: 0,
      usec: 500000
    };
    let expected = 50_f64;

    assert_eq!(calc_cpu_percent(dur, hz, &time), expected);
  }

}
