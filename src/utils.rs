
use sys_info;
use chrono::UTC;
use thread_id;

use std::collections::HashMap;

use super::*;

pub fn get_thread_id() -> usize {
  thread_id::get()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuTime {
  pub sec: u64,
  pub usec: u64
}

#[derive(Clone, Debug)]
pub struct History {
  process: Option<Stats>,
  // maps thread_id's to the last polled stats 
  thread: HashMap<usize, Stats>,
  // maps thread_id's to the last polled stats
  children: HashMap<usize, Stats>
}

impl Default for History {
  
  fn default() -> Self {
    History {
      process: None,
      thread: HashMap::new(),
      children: HashMap::new()
    }
  }

}

impl History {
  
  pub fn set_last(&mut self, kind: &StatType, poll: Stats) -> Option<Stats> {
    /// if thread or children, get thread_id
    let last = self.get_last(kind);
    
     match *kind {
      StatType::Process => { self.process = Some(poll); },
      StatType::Thread => { self.thread.insert(get_thread_id(), poll); },
      StatType::Children => { self.children.insert(get_thread_id(), poll); }
    };

    last
  }

  pub fn get_last(&self, kind: &StatType) -> Option<Stats> {
    match *kind {
      StatType::Process => self.process.clone(),
      StatType::Thread => self.thread.get(&get_thread_id()).cloned(),
      StatType::Children => self.children.get(&get_thread_id()).cloned()
    }
  }

}

pub fn safe_unsigned_sub(lhs: i64, rhs: i64) -> u64 {
  (lhs - rhs).wrapping_abs() as u64
}

pub fn calc_duration(kind: &StatType, history: &History, started: i64, polled: i64) -> u64 {
  let last = match history.get_last(kind) {
    Some(stats) => stats.polled,
    None => started
  };

  safe_unsigned_sub(last, polled)
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
  let cpu_time = (cpu.sec as f64) + (cpu.usec as f64 / 1000000_f64);
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

pub fn get_num_cores() -> Result<usize, Error> {
  match sys_info::cpu_num() {
    Ok(n) => Ok(n as usize),
    Err(e) => Err(Error::from(e))
  }
}

// ---------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn should_get_thread_id() {
    let id = get_thread_id();
    // FIXME make this smarter
    assert!(id > 0);
  }

  #[test]
  #[cfg(unix)]
  fn should_get_linux_platform() {
    assert_eq!(get_platform(), Ok(Platform::Linux));
  }

  #[test]
  #[cfg(windows)]
  fn should_get_windows_platform() {
    assert_eq!(get_platform(), Ok(Platform::Windows));
  }

  #[test]
  #[cfg(target_os="macos")]
  fn should_get_macos_platform() {
    assert_eq!(get_platform(), Ok(Platform::MacOS));
  }

  #[test]
  #[cfg(not(any(unix, windows, target_os="macos")))]
  fn should_get_unknown_platform() {
    assert_eq!(get_platform(), Ok(Platform::Unknown));
  }

  #[test]
  fn should_get_cpu_speed() {
    // FIXME make this smarter
    let speed = match get_cpu_speed() {
      Ok(s) => s,
      Err(e) => panic!("{:?}", e)
    };

    assert!(speed > 0);
  }

  #[test]
  fn should_get_num_cores() {
    // FIXME make this smarter
    let cores = match get_num_cores() {
      Ok(n) => n,
      Err(e) => panic!("{:?}", e)
    };

    assert!(cores > 0);
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

  #[test]
  fn should_create_empty_history() {
    let history = History::default();
    assert_eq!(history.process, None);
    assert!(history.thread.is_empty());
    assert!(history.children.is_empty());
  }

  #[test]
  fn should_set_last_process_history() {
    let mut history = History::default();
    let stats = Stats::new_empty(StatType::Process);
    let last = history.set_last(&StatType::Process, stats);
    assert_eq!(last, None);
  }

  #[test]
  fn should_set_last_thread_history() {
    let mut history = History::default();
    let stats = Stats::new_empty(StatType::Thread);
    let last = history.set_last(&StatType::Thread, stats);
    assert_eq!(last, None);
  }

  #[test]
  fn should_set_last_children_history() {
    let mut history = History::default();
    let stats = Stats::new_empty(StatType::Children);
    let last = history.set_last(&StatType::Children, stats);
    assert_eq!(last, None);
  }

  #[test]
  fn should_get_last_process_history() {
    let mut history = History::default();
    let last = history.get_last(&StatType::Process);
    assert_eq!(last, None);

    let stats = Stats::new_empty(StatType::Process);
    let last = history.set_last(&StatType::Process, stats.clone());
    assert_eq!(last, None);

    let last = history.get_last(&StatType::Process);
    assert!(last.is_some());
    let last_stats = last.unwrap();
    assert_eq!(last_stats, stats);
  }

  #[test]
  fn should_get_last_thread_history() {
    let mut history = History::default();
    let last = history.get_last(&StatType::Thread);
    assert_eq!(last, None);

    let stats = Stats::new_empty(StatType::Thread);
    let last = history.set_last(&StatType::Thread, stats.clone());
    assert_eq!(last, None);

    let last = history.get_last(&StatType::Thread);
    assert!(last.is_some());
    let last_stats = last.unwrap();
    assert_eq!(last_stats, stats);
  }

  #[test]
  fn should_get_last_children_history() {
    let mut history = History::default();
    let last = history.get_last(&StatType::Children);
    assert_eq!(last, None);

    let stats = Stats::new_empty(StatType::Children);
    let last = history.set_last(&StatType::Children, stats.clone());
    assert_eq!(last, None);

    let last = history.get_last(&StatType::Children);
    assert!(last.is_some());
    let last_stats = last.unwrap();
    assert_eq!(last_stats, stats);
  }

  #[test]
  fn should_calc_duration_with_started() {
    let history = History::default();
    let kind = StatType::Thread;
    let started = 1_i64;
    let polled = 1000_i64;

    let duration = calc_duration(&kind, &history, started, polled);
    assert_eq!(duration, (polled - started) as u64);
  }

  #[test]
  fn should_calc_duration_with_history() {
    let mut history = History::default();
    let kind = StatType::Thread;
    let started = 1_i64;
    let polled = 1000_i64;

    let stats = Stats::new_empty(kind.clone());
    // stats.polled will be 0
    history.set_last(&kind, stats.clone());

    let duration = calc_duration(&kind, &history, started, polled);
    assert_eq!(duration, (polled - stats.polled) as u64);
  }

  #[test]
  fn should_do_valid_safe_unsigned_sub() {
    let lhs = 100_i64;
    let rhs = 50_i64;
    let sub = safe_unsigned_sub(lhs, rhs);
    assert_eq!(sub, (lhs - rhs) as u64);
  }

  #[test]
  fn should_do_invalid_safe_unsigned_sub() {
    let lhs = 100_i64;
    let rhs = -50_i64;
    let sub = safe_unsigned_sub(lhs, rhs);
    assert_eq!(sub, 150_u64);
  }

  #[test]
  fn should_get_now_ms() {
    let now = now_ms();
    // FIXME make this smarter
    assert!(now > 0);
  }

}
