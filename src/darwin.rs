use libc::{
    integer_t, kern_return_t, mach_task_basic_info, mach_task_self, rusage, task_thread_times_info, time_value_t,
    timespec, timeval, EFAULT, EINVAL, EPERM, KERN_INVALID_ARGUMENT, KERN_SUCCESS, MACH_TASK_BASIC_INFO,
    MACH_TASK_BASIC_INFO_COUNT, RUSAGE_CHILDREN, RUSAGE_SELF, TASK_THREAD_TIMES_INFO, TASK_THREAD_TIMES_INFO_COUNT,
};
use std::mem::MaybeUninit;
use std::time::Duration;

// https://opensource.apple.com/source/xnu/xnu-792/osfmk/mach/mig_errors.h
pub const MIG_ARRAY_TOO_LARGE: kern_return_t = -307;

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
pub fn merge_thread_times_to_timespec(thread_times: task_thread_times_info) -> timespec {
    let user_time = Duration::from_micros(thread_times.user_time.microseconds as u64)
        + Duration::from_secs(thread_times.user_time.seconds as u64);

    let system_time = Duration::from_micros(thread_times.system_time.microseconds as u64)
        + Duration::from_secs(thread_times.system_time.seconds as u64);

    let total = user_time + system_time;

    time_value_t_to_timespec(time_value_t {
        seconds: total.as_secs() as integer_t,
        microseconds: total.subsec_micros() as integer_t,
    })
}

pub fn time_value_t_to_timespec(times: time_value_t) -> timespec {
    timespec {
        tv_sec: times.seconds as i64,
        tv_nsec: (times.microseconds * 1000) as i64,
    }
}

pub fn time_value_t_to_timeval(times: time_value_t) -> timeval {
    timeval {
        tv_sec: times.seconds as i64,
        tv_usec: times.microseconds,
    }
}

pub fn timespec_to_timeval(times: timespec) -> timeval {
    timeval {
        tv_sec: times.tv_sec,
        tv_usec: (times.tv_nsec / 1000) as i32,
    }
}

pub fn get_rusage_from_mach(usage: &mut rusage) -> Result<i32, SporkError> {
    let mut count = MACH_TASK_BASIC_INFO_COUNT;

    let mut basic_info: MaybeUninit<mach_task_basic_info> = MaybeUninit::zeroed();
    map_mach_resp(unsafe {
        libc::task_info(
            mach_task_self(),
            MACH_TASK_BASIC_INFO,
            basic_info.as_mut_ptr().cast(),
            &mut count,
        )
    })?;

    let basic_info = unsafe { basic_info.assume_init() };

    usage.ru_maxrss = basic_info.resident_size as i64 / 1024;
    usage.ru_stime = time_value_t_to_timeval(basic_info.system_time);

    Ok(KERN_SUCCESS)
}

// this should always be called before get_stats since they both consume the clock
pub fn get_thread_cpu_time() -> Result<timespec, SporkError> {
    let mut thread_times: MaybeUninit<task_thread_times_info> = MaybeUninit::zeroed();
    let mut thread_times_count = TASK_THREAD_TIMES_INFO_COUNT;
    let _ = map_mach_resp(unsafe {
        libc::task_info(
            mach_task_self(),
            TASK_THREAD_TIMES_INFO,
            thread_times.as_mut_ptr().cast(),
            &mut thread_times_count,
        )
    })?;

    let thread_times = unsafe { thread_times.assume_init() };

    // Appears the Linux equivalent to this actually is a combination of CPU and USER times
    // Ok(time_value_t_to_timespec(&(thread_times.user_time)))
    // For now lets combine (Which is what clock_gettime appears to do)
    Ok(merge_thread_times_to_timespec(thread_times))
}

pub fn get_stats(kind: &StatType) -> Result<rusage, SporkError> {
    let (t_times, code): (Option<timespec>, Option<i32>) = match *kind {
        StatType::Process => (None, Some(RUSAGE_SELF)),
        StatType::Children => (None, Some(RUSAGE_CHILDREN)),
        StatType::Thread => (Some(get_thread_cpu_time()?), None),
    };

    let mut usage = MaybeUninit::zeroed();
    if let Some(r_code) = code {
        let _ = map_posix_resp(unsafe { libc::getrusage(r_code, usage.as_mut_ptr()) })?;
    } else {
        let _ = unsafe { get_rusage_from_mach(usage.assume_init_mut())? };
    }

    let mut usage = unsafe { usage.assume_init() };

    if let Some(t_times) = t_times {
        // use clock_gettime results for threads
        usage.ru_utime = timespec_to_timeval(t_times);
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

/// Poke the maximum CPU frequency from IOReg on Apple Silicon systems in Hz.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn poke_apple_silicon_cpu_freq() -> Result<u32, SporkError> {
    use apple_sys::IOKit::{
        kCFAllocatorDefault, kCFAllocatorNull, kIOMainPortDefault, CFDataGetBytes, CFDataGetLength, CFIndex, CFRange,
        CFRelease, CFStringBuiltInEncodings_kCFStringEncodingUTF8, CFStringCreateWithBytesNoCopy, IOIteratorNext,
        IOObjectRelease, IORegistryEntryCreateCFProperty, IORegistryEntryGetName, IOServiceGetMatchingServices,
        IOServiceMatching,
    };

    use std::ffi::CStr;
    const VOLTAGE_STATE5_SRAM: &[u8] = b"voltage-states5-sram";

    // SAFETY: IOServiceMatching accepts a C string for name.
    // https://developer.apple.com/documentation/iokit/1514687-ioservicematching
    let matching = unsafe {
        let matching = IOServiceMatching(b"AppleARMIODevice\0".as_ptr().cast());
        if matching.is_null() {
            return Err(SporkError::unimplemented());
        }
        matching
    };

    let mut iter = 0;
    // SAFETY: matching has been checked to be not null. iter is always initialized and aligned.
    let _ = map_mach_resp(unsafe { IOServiceGetMatchingServices(kIOMainPortDefault, matching, &mut iter) })?;

    // io_name_t is defined as an array of 128 chars.
    // https://developer.apple.com/documentation/kernel/io_name_t
    let mut name = [0u8; 128];

    let entry = 'entry: {
        loop {
            // SAFETY: Even if iter is invalid, this is safe to call, and will return 0.
            let entry = unsafe { IOIteratorNext(iter) };
            if entry == 0 {
                break;
            }
            // SAFETY: name is io_name_t, which has size of 128 chars. It remains alive until
            // the end of this function.
            if map_mach_resp(unsafe { IORegistryEntryGetName(entry, name.as_mut_ptr().cast()) }).is_err() {
                // SAFETY: An error occurred so we must release the current entry.
                unsafe {
                    IOObjectRelease(entry);
                    continue;
                }
            }

            // IORegistryEntryName always returns a C-string name assigned to a registry entry,
            // if there is no error.
            // https://developer.apple.com/documentation/iokit/1514323-ioregistryentrygetname
            let name = CStr::from_bytes_until_nul(&name).expect("kernel iterator should always return null-terminator");
            if name.to_bytes() == b"pmgr" {
                break 'entry entry;
            } else {
                // SAFETY: We are finished with the entry and should release it.
                unsafe {
                    IOObjectRelease(entry);
                }
            }
        }

        return Err(SporkError::unimplemented());
    };

    let p_core_ref = unsafe {
        // SAFETY: CFStringCreateWithBytesNoCopy does not accept null-terminators,
        // and does not need to be deallocated since we're using the static string
        // as the backing memory.
        let prop_name = CFStringCreateWithBytesNoCopy(
            kCFAllocatorDefault,
            VOLTAGE_STATE5_SRAM.as_ptr(),
            VOLTAGE_STATE5_SRAM.len() as CFIndex,
            CFStringBuiltInEncodings_kCFStringEncodingUTF8,
            0,
            kCFAllocatorNull,
        );

        // SAFETY: CFStringCreateWithBytesNoCopy is infallible, so prop_name is always valid.
        IORegistryEntryCreateCFProperty(entry, prop_name, kCFAllocatorDefault, 0)
    };

    if p_core_ref.is_null() {
        return Err(SporkError::unimplemented());
    }

    // SAFETY: p_core_ref has been checked for null.
    let p_core_len = unsafe { CFDataGetLength(p_core_ref.cast()) };
    if p_core_len < 8 {
        return Err(SporkError::unimplemented());
    }

    let range = CFRange {
        location: p_core_len - 8,
        length: 4,
    };

    let mut cpu_freq = [0u8; 4];
    // SAFETY: p_core_len must be at least 8 bytes as that is the minimum length of
    // two integers that represent a frequency/power state tuple.
    // Therefore, the 4 byte range requested, from 8 bytes before the end of the buffer,
    // always falls within the buffer of the data p_core_ref points to.
    unsafe { CFDataGetBytes(p_core_ref.cast(), range, cpu_freq.as_mut_ptr()) }

    // SAFETY: p_core_ref is not null. entry and iter are invalid after
    // this block, and are no longer used.
    unsafe {
        CFRelease(p_core_ref);
        IOObjectRelease(entry);
        IOObjectRelease(iter);
    }

    Ok(u32::from_le_bytes(cpu_freq))
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
