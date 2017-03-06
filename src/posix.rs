
use libc;
use libc::RUSAGE_SELF;
use libc::RUSAGE_CHILDREN;
use libc::RUSAGE_THREAD;
use libc::timeval;
use libc::rusage;

use super::*;

use utils;
use utils::{CpuTime, History};

use std::io::Read;
use std::fs::File;

fn empty_rusage() -> rusage {
  libc::rusage {
    ru_utime: timeval {
      tv_sec: 0,
      tv_usec: 0
    },
    ru_stime: timeval {
      tv_sec: 0,
      tv_usec: 0
    },
    ru_maxrss: 0,
    ru_ixrss: 0,
    ru_idrss: 0,
    ru_isrss: 0,
    ru_minflt: 0,
    ru_majflt: 0,
    ru_nswap: 0,
    ru_inblock: 0,
    ru_oublock: 0,
    ru_msgsnd: 0,
    ru_msgrcv: 0,
    ru_nsignals: 0,
    ru_nvcsw: 0,
    ru_nivcsw: 0 
  }
}

pub fn get_clock_ticks() -> Result<i64, Error> {
  Ok(unsafe { libc::sysconf(libc::_SC_CLK_TCK) })
}

pub fn get_stats(kind: &StatType) -> rusage {
  let code = match *kind {
    StatType::Process => RUSAGE_SELF,
    StatType::Thread => RUSAGE_THREAD,
    StatType::Children => RUSAGE_CHILDREN
  };

  let mut usage = empty_rusage();
  unsafe {
    libc::getrusage(code, &mut usage);
  }
  usage
}

pub fn get_cpu_percent(kind: &StatType, hz: u64, duration: u64, val: &rusage) -> f64 {
  let times = CpuTime {
    sec: (val.ru_stime.tv_sec + val.ru_utime.tv_sec).wrapping_abs() as u64,
    usec: (val.ru_stime.tv_usec + val.ru_utime.tv_usec).wrapping_abs() as u64
  };
  println!("get cpu percent, hz: {:?}, duration: {:?}, times: {:?}", hz, duration, times);

  utils::calc_cpu_percent(duration, hz, &times)
}

// -----------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use utils::*;

  fn format_timeval(val: &timeval) -> String {
    format!(
      "timeval {{ tv_sec: {:?}, tv_usec: {:?} }}",
      val.tv_sec, val.tv_usec 
    )
  }

  fn print_timeval(val: &timeval) {
    println!("{:?}", format_timeval(val));
  }

  fn format_rusage(usage: &rusage) -> String {
    format!("rusage {{ ru_utime: {:?}, ru_stime: {:?}, ru_maxrss: {:?}, ru_ixrss: {:?}, ru_idrss: {:?}, ru_isrss: {:?}, ru_minflt: {:?}, ru_majflt: {:?}, ru_nswap: {:?}, ru_inblock: {:?}, ru_oublock: {:?}, ru_msgsnd: {:?}, ru_msgrcv: {:?}, ru_nsignals: {:?}, ru_nvcsw: {:?}, ru_nivcsw: {:?} }}", 
      format_timeval(&usage.ru_utime), format_timeval(&usage.ru_stime), usage.ru_maxrss, usage.ru_ixrss, usage.ru_idrss, usage.ru_isrss, usage.ru_minflt, usage.ru_majflt, usage.ru_nswap, usage.ru_inblock,
      usage.ru_oublock, usage.ru_msgsnd, usage.ru_msgrcv, usage.ru_nsignals, usage.ru_nvcsw, 
      usage.ru_nivcsw
    )
  }

  fn print_rusage(usage: &rusage) {
    println!("{:?}", format_rusage(usage));
  }

  fn fib(n: u64) -> u64 {
    if n > 2 {
      fib(n - 1) + fib(n - 2) 
    } else {
      1
    }
  }

  #[test]
  fn should_get_clock_ticks() {
    let ticks = get_clock_ticks().unwrap();
    assert!(ticks > 0);
  }

  #[test]
  fn should_poll_process_stats() {
    let kind = StatType::Process;
    let usage = get_stats(&kind);
    print_rusage(&usage);
  }

  #[test]
  fn should_poll_thread_stats() {
    let kind = StatType::Thread;
    fib(10);
    let usage = get_stats(&kind);
    print_rusage(&usage);
  }

  #[test]
  fn should_poll_children_stats() {
    let kind = StatType::Children;
    let usage = get_stats(&kind);
    print_rusage(&usage);
  }

  #[test]
  fn should_poll_cpu() {
    let kind = StatType::Thread;
    let hz = utils::get_cpu_speed().unwrap();
    let last_rusage = get_stats(&kind);
    let started = utils::now_ms();
    fib(43);
    let finished = utils::now_ms();
    let rusage = get_stats(&kind);
    let duration = utils::safe_unsigned_sub(finished, started); 
    let cpu = get_cpu_percent(&kind, hz, duration, &rusage);
  
    assert!(cpu > 98.0);
  }




}
