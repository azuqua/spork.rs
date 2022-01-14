use chrono::UTC;
use sys_info;
use thread_id;

use libc::timespec;

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use super::*;

pub fn get_thread_id() -> usize {
    thread_id::get()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuTime {
    pub sec: u64,
    pub usec: u64,
}

#[derive(Clone, Debug)]
pub struct History {
    process: RefCell<Option<Stats>>,
    // maps thread_id's to the last polled stats
    thread: RefCell<HashMap<usize, Stats>>,
    // maps thread_id's to the last polled stats
    children: RefCell<HashMap<usize, Stats>>,
}

impl Default for History {
    fn default() -> Self {
        History {
            process: RefCell::new(None),
            thread: RefCell::new(HashMap::new()),
            children: RefCell::new(HashMap::new()),
        }
    }
}

impl History {
    pub fn set_last(&self, kind: &StatType, poll: Stats) -> Option<Stats> {
        let last = self.get_last(kind);

        match *kind {
            StatType::Process => {
                let mut process = self.process.borrow_mut();
                let process_ref = process.deref_mut();

                *process_ref = Some(poll);
            }
            StatType::Thread => {
                let mut threads = self.thread.borrow_mut();
                let threads_ref = threads.borrow_mut();

                threads_ref.insert(get_thread_id(), poll);
            }
            StatType::Children => {
                let mut children = self.children.borrow_mut();
                let children_ref = children.borrow_mut();

                children_ref.insert(get_thread_id(), poll);
            }
        };

        last
    }

    pub fn get_last(&self, kind: &StatType) -> Option<Stats> {
        match *kind {
            StatType::Process => self.process.clone().into_inner(),
            StatType::Thread => {
                let t_id = get_thread_id();
                let threads = self.thread.borrow();
                let threads_ref = threads.deref();

                threads_ref.get(&t_id).cloned()
            }
            StatType::Children => {
                let t_id = get_thread_id();
                let children = self.children.borrow();
                let children_ref = children.deref();

                children_ref.get(&t_id).cloned()
            }
        }
    }

    pub fn clear_last(&self, kind: &StatType) -> Option<Stats> {
        let last = self.get_last(kind);

        match *kind {
            StatType::Process => {
                let mut process = self.process.borrow_mut();
                let process_ref = process.deref_mut();

                *process_ref = None;
            }
            StatType::Thread => {
                let mut threads = self.thread.borrow_mut();
                let threads_ref = threads.borrow_mut();

                threads_ref.remove(&get_thread_id());
            }
            StatType::Children => {
                let mut children = self.children.borrow_mut();
                let children_ref = children.borrow_mut();

                children_ref.remove(&get_thread_id());
            }
        };

        last
    }
}

pub fn safe_unsigned_sub(lhs: i64, rhs: i64) -> u64 {
    (lhs - rhs).wrapping_abs() as u64
}

pub fn calc_duration(kind: &StatType, history: &History, started: i64, polled: i64) -> u64 {
    let last = match history.get_last(kind) {
        Some(stats) => stats.polled,
        None => started,
    };

    safe_unsigned_sub(last, polled)
}

pub fn now_ms() -> i64 {
    let now = UTC::now();
    (now.timestamp() * 1000 + (now.timestamp_subsec_millis() as i64)) as i64
}

pub fn get_platform() -> Result<Platform, SporkError> {
    match sys_info::os_type()?.as_ref() {
        "Linux" => Ok(Platform::Linux),
        "Windows" => Ok(Platform::Windows),
        "Darwin" => Ok(Platform::MacOS),
        _ => Ok(Platform::Unknown),
    }
}

pub fn calc_cpu_percent(history: &History, kind: &StatType, curr_cpu_time: f64, duration: u64) -> f64 {
    let prev_cpu_time = match history.get_last(kind) {
        Some(stats) => stats.cpu_time,
        None => 0_f64,
    };
    let cpu_time_delta = curr_cpu_time - prev_cpu_time;
    ((cpu_time_delta / duration as f64) * 1000 as f64) * 100 as f64
}

pub fn get_cpu_speed() -> Result<u64, SporkError> {
    match sys_info::cpu_speed() {
        Ok(s) => Ok(s * 1000),
        Err(e) => Err(SporkError::from(e)),
    }
}

pub fn get_num_cores() -> Result<usize, SporkError> {
    match sys_info::cpu_num() {
        Ok(n) => Ok(n as usize),
        Err(e) => Err(SporkError::from(e)),
    }
}

// Not actually dead - but cargo thinks it is (Used in tests)
#[allow(dead_code)]
pub fn empty_timespec() -> timespec {
    timespec { tv_sec: 0, tv_nsec: 0 }
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
    #[cfg(target_os = "linux")]
    fn should_get_linux_platform() {
        assert_eq!(get_platform(), Ok(Platform::Linux));
    }

    #[test]
    #[cfg(windows)]
    fn should_get_windows_platform() {
        assert_eq!(get_platform(), Ok(Platform::Windows));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn should_get_macos_platform() {
        assert_eq!(get_platform(), Ok(Platform::MacOS));
    }

    #[test]
    #[cfg(not(any(unix, windows, target_os = "macos")))]
    fn should_get_unknown_platform() {
        assert_eq!(get_platform(), Ok(Platform::Unknown));
    }

    #[test]
    fn should_get_cpu_speed() {
        let speed = match get_cpu_speed() {
            Ok(s) => s,
            Err(e) => panic!("{:?}", e),
        };

        assert!(speed > 0);
    }

    #[test]
    fn should_get_num_cores() {
        let cores = match get_num_cores() {
            Ok(n) => n,
            Err(e) => panic!("{:?}", e),
        };

        assert!(cores > 0);
    }

    #[test]
    fn should_create_empty_history() {
        let history = History::default();
        assert_eq!(history.process.into_inner(), None);
        assert!(history.thread.into_inner().is_empty());
        assert!(history.children.into_inner().is_empty());
    }

    #[test]
    fn should_set_last_process_history() {
        let history = History::default();
        let stats = Stats::new_empty(StatType::Process);
        let last = history.set_last(&StatType::Process, stats);
        assert_eq!(last, None);
    }

    #[test]
    fn should_set_last_thread_history() {
        let history = History::default();
        let stats = Stats::new_empty(StatType::Thread);
        let last = history.set_last(&StatType::Thread, stats);
        assert_eq!(last, None);
    }

    #[test]
    fn should_set_last_children_history() {
        let history = History::default();
        let stats = Stats::new_empty(StatType::Children);
        let last = history.set_last(&StatType::Children, stats);
        assert_eq!(last, None);
    }

    #[test]
    fn should_get_last_process_history() {
        let history = History::default();
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
        let history = History::default();
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
        let history = History::default();
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
        let history = History::default();
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
    fn should_clear_process_history() {
        let history = History::default();
        let kind = StatType::Process;
        let stats = Stats::new_empty(kind.clone());

        let last = history.set_last(&kind, stats.clone());
        assert_eq!(last, None);

        let cleared = history.clear_last(&kind);
        assert_eq!(cleared, Some(stats));

        let empty = history.get_last(&kind);
        assert_eq!(empty, None);
    }

    #[test]
    fn should_clear_thread_history() {
        let history = History::default();
        let kind = StatType::Thread;
        let stats = Stats::new_empty(kind.clone());

        let last = history.set_last(&kind, stats.clone());
        assert_eq!(last, None);

        let cleared = history.clear_last(&kind);
        assert_eq!(cleared, Some(stats));

        let empty = history.get_last(&kind);
        assert_eq!(empty, None);
    }

    #[test]
    fn should_clear_children_history() {
        let history = History::default();
        let kind = StatType::Children;
        let stats = Stats::new_empty(kind.clone());

        let last = history.set_last(&kind, stats.clone());
        assert_eq!(last, None);

        let cleared = history.clear_last(&kind);
        assert_eq!(cleared, Some(stats));

        let empty = history.get_last(&kind);
        assert_eq!(empty, None);
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
        assert!(now > 0);
    }
}
