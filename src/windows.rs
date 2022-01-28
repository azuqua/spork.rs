use std;

use kernel32;
use winapi;
use winapi::psapi::PROCESS_MEMORY_COUNTERS;

use utils::CpuTime;

use super::*;

fn get_thread_handle() -> winapi::HANDLE {
    unsafe { kernel32::GetCurrentThread() }
}

fn get_current_process() -> winapi::HANDLE {
    unsafe { kernel32::GetCurrentProcess() }
}

fn empty_filetime() -> winapi::FILETIME {
    winapi::minwindef::FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    }
}

fn empty_proc_mem_counters() -> PROCESS_MEMORY_COUNTERS {
    PROCESS_MEMORY_COUNTERS {
        cb: 0,
        PageFaultCount: 0,
        PeakWorkingSetSize: 0,
        WorkingSetSize: 0,
        QuotaPeakPagedPoolUsage: 0,
        QuotaPagedPoolUsage: 0,
        QuotaPeakNonPagedPoolUsage: 0,
        QuotaNonPagedPoolUsage: 0,
        PagefileUsage: 0,
        PeakPagefileUsage: 0,
    }
}

// convert the two 32 bit ints in a FILETIME a u64
fn wtf(f: winapi::minwindef::FILETIME) -> u64 {
    (f.dwLowDateTime + (2 << 31) * f.dwHighDateTime) as u64
}

pub fn get_mem_stats(kind: &StatType) -> Result<PROCESS_MEMORY_COUNTERS, SporkError> {
    match *kind {
        StatType::Process => {
            let handle = get_current_process();
            let mut memory = empty_proc_mem_counters();
            let cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

            unsafe {
                kernel32::K32GetProcessMemoryInfo(handle, &mut memory, cb);
            };

            Ok(memory)
        }
        StatType::Thread => {
            let handle = get_thread_handle();
            let mut memory = empty_proc_mem_counters();
            let cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

            unsafe {
                kernel32::K32GetProcessMemoryInfo(handle, &mut memory, cb);
            };

            Ok(memory)
        }
        StatType::Children => Err(SporkError::new(
            SporkErrorKind::Unimplemented,
            "Windows child thread memory stat not yet implemented!".to_owned(),
        )),
    }
}

#[derive(Debug)]
pub struct WindowsCpuStats {
    creation: u64,
    exit: u64,
    kernel: u64,
    user: u64,
}

pub fn get_cpu_times(kind: &StatType) -> Result<WindowsCpuStats, SporkError> {
    match *kind {
        StatType::Process => {
            let handle = get_current_process();
            let mut lp_creation_time = empty_filetime();
            let mut lp_exit_time = empty_filetime();
            let mut lp_kernal_time = empty_filetime();
            let mut lp_user_time = empty_filetime();

            unsafe {
                kernel32::GetProcessTimes(
                    handle,
                    &mut lp_creation_time,
                    &mut lp_exit_time,
                    &mut lp_kernal_time,
                    &mut lp_user_time,
                );
            };

            Ok(WindowsCpuStats {
                creation: wtf(lp_creation_time),
                exit: wtf(lp_exit_time),
                kernel: wtf(lp_kernal_time),
                user: wtf(lp_user_time),
            })
        }
        StatType::Thread => {
            let handle = get_thread_handle();
            let mut lp_creation_time = empty_filetime();
            let mut lp_exit_time = empty_filetime();
            let mut lp_kernal_time = empty_filetime();
            let mut lp_user_time = empty_filetime();

            unsafe {
                kernel32::GetThreadTimes(
                    handle,
                    &mut lp_creation_time,
                    &mut lp_exit_time,
                    &mut lp_kernal_time,
                    &mut lp_user_time,
                );
            };

            Ok(WindowsCpuStats {
                creation: wtf(lp_creation_time),
                exit: wtf(lp_exit_time),
                kernel: wtf(lp_kernal_time),
                user: wtf(lp_user_time),
            })
        }
        StatType::Children => Err(SporkError::new(
            SporkErrorKind::Unimplemented,
            "Windows child thread memory stat not yet implemented!".to_owned(),
        )),
    }
}

pub fn combine_cpu_times(val: &WindowsCpuStats) -> f64 {
    // Kernal/User time here are in 100ns units. Divide by 10,000,000 to convert
    let times = CpuTime {
        sec: (val.kernel / 10000000 + val.user / 10000000) as u64,
        usec: (val.kernel % 10000000 / 10 + val.user % 10000000 / 10) as u64,
    };

    (times.sec as f64) + (times.usec as f64 / 1000000_f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_empty_file_time() {
        let times = empty_filetime();
        assert_eq!(times.dwLowDateTime, 0);
        assert_eq!(times.dwHighDateTime, 0);
    }

    #[test]
    fn should_poll_cpu_process_stats() {
        let kind = StatType::Process;
        let usage = get_cpu_times(&kind);
        assert!(usage.is_ok())
    }

    #[test]
    fn should_poll_cpu_memory_stats() {
        let kind = StatType::Process;
        let usage = get_mem_stats(&kind);
        assert!(usage.is_ok())
    }

    #[test]
    fn should_poll_thread_process_stats() {
        let kind = StatType::Thread;
        let usage = get_cpu_times(&kind);
        assert!(usage.is_ok())
    }

    #[test]
    fn should_poll_thread_memory_stats() {
        let kind = StatType::Thread;
        let usage = get_mem_stats(&kind);
        assert!(usage.is_ok())
    }

    #[test]
    fn should_poll_child_cpu_stats() {
        let kind = StatType::Children;
        let usage = get_cpu_times(&kind);
        match usage {
            Ok(_) => panic!("Should of returned spork error"),
            Err(err) => match err.kind {
                SporkErrorKind::Unimplemented => assert!(true),
                _ => panic!("Wrong error returnd from child process stats failure"),
            },
        }
    }

    #[test]
    fn should_poll_child_memory_stats() {
        let kind = StatType::Children;
        let usage = get_mem_stats(&kind);
        match usage {
            Ok(_) => panic!("Should of returned spork error"),
            Err(err) => match err.kind {
                SporkErrorKind::Unimplemented => assert!(true),
                _ => panic!("Wrong error returnd from child process stats failure"),
            },
        }
    }
}
