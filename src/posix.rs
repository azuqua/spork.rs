
use libc;
use libc::{CLOCK_THREAD_CPUTIME_ID, EFAULT, EINVAL, EPERM, RUSAGE_CHILDREN, RUSAGE_SELF, RUSAGE_THREAD};
use libc::timespec;
use libc::timeval;
use libc::rusage;

use super::*;

use utils::CpuTime;
use utils::empty_timespec;

fn empty_rusage() -> rusage {
    libc::rusage {
        ru_utime: timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        ru_stime: timeval {
            tv_sec: 0,
            tv_usec: 0,
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
        ru_nivcsw: 0,
    }
}

fn map_posix_resp(code: i32) -> Result<i32, SporkError> {
    match code {
        EFAULT => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Invalid timespec address space.",
        )),
        EINVAL => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Invalid clock ID.",
        )),
        EPERM => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Invalid clock permissions.",
        )),
        _ => Ok(code),
    }
}

#[allow(dead_code)]
pub fn get_clock_ticks() -> Result<i64, SporkError> {
    Ok(unsafe { libc::sysconf(libc::_SC_CLK_TCK) })
}

#[allow(dead_code)]
pub fn timespec_to_cpu_time(times: &timespec) -> CpuTime {
    CpuTime {
        sec: times.tv_sec.wrapping_abs() as u64,
        usec: (times.tv_nsec.wrapping_abs() / 1000) as u64,
    }
}

pub fn timespec_to_timeval(times: &timespec) -> timeval {
    timeval {
        tv_sec: times.tv_sec,
        tv_usec: times.tv_nsec / 1000,
    }
}

// this should always be called before get_stats since they both consume the clock
pub fn get_thread_cpu_time() -> Result<timespec, SporkError> {
    let mut times = empty_timespec();
    let _ = try!(map_posix_resp(unsafe {
        libc::clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut times)
    }));

    Ok(times)
}

pub fn get_stats(kind: &StatType) -> Result<rusage, SporkError> {
    let (t_times, code): (Option<timespec>, i32) = match *kind {
        StatType::Process => (None, RUSAGE_SELF),
        StatType::Children => (None, RUSAGE_CHILDREN),
        StatType::Thread => (Some(try!(get_thread_cpu_time())), RUSAGE_THREAD),
    };

    let mut usage = empty_rusage();
    let _ = try!(map_posix_resp(unsafe { libc::getrusage(code, &mut usage) }));

    if t_times.is_some() {
        // use clock_gettime results for threads
        usage.ru_utime = timespec_to_timeval(&t_times.unwrap());
    }

    Ok(usage)
}

pub fn get_cpu_time(val: &rusage) -> f64 {
    let times = CpuTime {
        sec: (val.ru_stime.tv_sec + val.ru_utime.tv_sec).wrapping_abs() as u64,
        usec: (val.ru_stime.tv_usec + val.ru_utime.tv_usec).wrapping_abs() as u64,
    };

    (times.sec as f64) + (times.usec as f64 / 1000000_f64)
}

// -----------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use utils::empty_timespec;

    fn format_timeval(val: &timeval) -> String {
        format!(
            "timeval {{ tv_sec: {:?}, tv_usec: {:?} }}",
            val.tv_sec,
            val.tv_usec
        )
    }

    #[allow(dead_code)]
    fn print_timeval(val: &timeval) {
        println!("{:?}", format_timeval(val));
    }

    fn format_rusage(usage: &rusage) -> String {
        format!(
            "rusage {{ ru_utime: {:?}, ru_stime: {:?}, ru_maxrss: {:?}, ru_ixrss: {:?}, ru_idrss: {:?}, ru_isrss: {:?}, ru_minflt: {:?}, ru_majflt: {:?}, ru_nswap: {:?}, ru_inblock: {:?}, ru_oublock: {:?}, ru_msgsnd: {:?}, ru_msgrcv: {:?}, ru_nsignals: {:?}, ru_nvcsw: {:?}, ru_nivcsw: {:?} }}",
            format_timeval(&usage.ru_utime),
            format_timeval(&usage.ru_stime),
            usage.ru_maxrss,
            usage.ru_ixrss,
            usage.ru_idrss,
            usage.ru_isrss,
            usage.ru_minflt,
            usage.ru_majflt,
            usage.ru_nswap,
            usage.ru_inblock,
            usage.ru_oublock,
            usage.ru_msgsnd,
            usage.ru_msgrcv,
            usage.ru_nsignals,
            usage.ru_nvcsw,
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
    fn should_get_empty_timespec() {
        let times = empty_timespec();
        assert_eq!(times.tv_sec, 0);
        assert_eq!(times.tv_nsec, 0);
    }

    #[test]
    fn should_convert_timespec_to_cpu_time() {
        let mut times = empty_timespec();
        times.tv_sec = 1;
        times.tv_nsec = 10000;

        let cpu = timespec_to_cpu_time(&times);
        assert_eq!(times.tv_sec as u64, cpu.sec);
        assert_eq!(times.tv_nsec as u64, cpu.usec * 1000);
    }

    #[test]
    fn should_get_empty_rusage() {
        let usage = empty_rusage();
        assert_eq!(usage.ru_maxrss, 0);
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
        print_rusage(&usage.unwrap());
    }

    #[test]
    fn should_poll_thread_stats() {
        let kind = StatType::Thread;
        fib(10);
        let usage = get_stats(&kind);
        print_rusage(&usage.unwrap());
    }

    #[test]
    fn should_poll_children_stats() {
        let kind = StatType::Children;
        let usage = get_stats(&kind);
        print_rusage(&usage.unwrap());
    }

    #[test]
    fn should_get_thread_cpu_times() {
        let times = match get_thread_cpu_time() {
            Ok(t) => t,
            Err(e) => panic!("Error getting thread cpu times {:?}", e),
        };

        assert!(times.tv_sec >= 0);
        assert!(times.tv_nsec >= 0);
    }

}
