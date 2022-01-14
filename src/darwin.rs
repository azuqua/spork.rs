use mach::task::task_info;
use mach::task_info::{
    task_basic_info, task_basic_info_t, task_thread_times_info, task_thread_times_info_t, MIG_ARRAY_TOO_LARGE,
    TASK_BASIC_INFO, TASK_BASIC_INFO_COUNT, TASK_THREAD_TIMES_INFO, TASK_THREAD_TIMES_INFO_COUNT,
};

use mach::time_value::{time_value_add, time_value_t};

use mach::traps::mach_task_self;

use mach::kern_return::{KERN_INVALID_ARGUMENT, KERN_SUCCESS};

use libc;
use libc::rusage;
use libc::timespec;
use libc::timeval;
use libc::{EFAULT, EINVAL, EPERM, RUSAGE_CHILDREN, RUSAGE_SELF};

use super::*;

use utils::CpuTime;

fn map_posix_resp(code: i32) -> Result<i32, SporkError> {
    match code {
        EFAULT => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Invalid timespec address space.",
        )),
        EINVAL => Err(SporkError::new_borrowed(SporkErrorKind::Unknown, "Invalid clock ID.")),
        EPERM => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Invalid clock permissions.",
        )),
        _ => Ok(code),
    }
}

fn map_mach_resp(code: libc::c_int) -> Result<i32, SporkError> {
    match code {
        KERN_SUCCESS => Ok(code),
        KERN_INVALID_ARGUMENT => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Target task is not a thread or flavor not recognized",
        )),
        MIG_ARRAY_TOO_LARGE => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Target array too small",
        )),
        _ => Err(SporkError::new_borrowed(
            SporkErrorKind::Unknown,
            "Unknown error had occured",
        )),
    }
}

#[allow(dead_code)]
pub fn merge_thread_times_to_timespec(thread_times: &task_thread_times_info) -> timespec {
    let mut time_total = time_value_t {
        seconds: 0,
        microseconds: 0,
    };
    time_value_add(&mut time_total, &(thread_times.user_time));
    time_value_add(&mut time_total, &(thread_times.system_time));
    time_value_t_to_timespec(&time_total)
}

pub fn time_value_t_to_timespec(times: &time_value_t) -> timespec {
    timespec {
        tv_sec: times.seconds as i64,
        tv_nsec: (times.microseconds * 1000) as i64,
    }
}

pub fn time_value_t_to_timeval(times: &time_value_t) -> timeval {
    timeval {
        tv_sec: times.seconds as i64,
        tv_usec: times.microseconds,
    }
}

pub fn timespec_to_timeval(times: &timespec) -> timeval {
    timeval {
        tv_sec: times.tv_sec,
        tv_usec: (times.tv_nsec / 1000) as i32,
    }
}

pub fn get_rusage_from_mach(usage: &mut rusage) -> Result<i32, SporkError> {
    let mut basic_info = task_basic_info::new();
    let basic_info_ptr = (&mut basic_info as task_basic_info_t) as libc::uintptr_t;
    let mut count = TASK_BASIC_INFO_COUNT;

    map_mach_resp(unsafe { task_info(mach_task_self(), TASK_BASIC_INFO, basic_info_ptr, &mut count) })?;
    usage.ru_maxrss = basic_info.resident_size as i64 / 1024;
    usage.ru_stime = time_value_t_to_timeval(&basic_info.system_time);

    Ok(KERN_SUCCESS)
}

// this should always be called before get_stats since they both consume the clock
pub fn get_thread_cpu_time() -> Result<timespec, SporkError> {
    let mut thread_times = task_thread_times_info::new();
    let thread_times_ptr = (&mut thread_times as task_thread_times_info_t) as libc::uintptr_t;
    let mut thread_times_count = TASK_THREAD_TIMES_INFO_COUNT;
    let _ = map_mach_resp(unsafe {
        task_info(
            mach_task_self(),
            TASK_THREAD_TIMES_INFO,
            thread_times_ptr,
            &mut thread_times_count,
        )
    })?;

    // Appears the Linux equivilent to this actually is a compination of CPU and USER times
    // Ok(time_value_t_to_timespec(&(thread_times.user_time)))
    // For now lets combine (Which is what clock_gettime appears to do)
    Ok(merge_thread_times_to_timespec(&thread_times))
}

pub fn get_stats(kind: &StatType) -> Result<rusage, SporkError> {
    let (t_times, code): (Option<timespec>, Option<i32>) = match *kind {
        StatType::Process => (None, Some(RUSAGE_SELF)),
        StatType::Children => (None, Some(RUSAGE_CHILDREN)),
        StatType::Thread => (Some(get_thread_cpu_time()?), None),
    };

    let mut usage = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    if let Some(r_code) = code {
        let _ = map_posix_resp(unsafe { libc::getrusage(r_code, &mut usage) })?;
    } else {
        let _ = get_rusage_from_mach(&mut usage)?;
    }

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

    fn get_clock_ticks() -> Result<i64, SporkError> {
        Ok(unsafe { libc::sysconf(libc::_SC_CLK_TCK) })
    }

    fn timespec_to_cpu_time(times: &timespec) -> CpuTime {
        CpuTime {
            sec: times.tv_sec.wrapping_abs() as u64,
            usec: (times.tv_nsec.wrapping_abs() / 1000) as u64,
        }
    }

    fn format_timeval(val: &timeval) -> String {
        format!("timeval {{ tv_sec: {:?}, tv_usec: {:?} }}", val.tv_sec, val.tv_usec)
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
            Err(e) => panic!("SporkError getting thread cpu times {:?}", e),
        };

        assert!(times.tv_sec >= 0);
        assert!(times.tv_nsec >= 0);
    }
}
