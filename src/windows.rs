use std;
use std::mem::MaybeUninit;

use num_cpus::get;
use windows_sys::Win32::Foundation::{GetLastError, FILETIME, HANDLE};
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentThread, GetProcessTimes, GetThreadTimes};

use utils::CpuTime;

use super::*;

fn get_thread_handle() -> HANDLE {
    unsafe { GetCurrentThread() }
}

fn get_current_process() -> HANDLE {
    unsafe { GetCurrentProcess() }
}

fn empty_filetime() -> FILETIME {
    FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    }
}

// convert the two 32 bit ints in a FILETIME a u64
fn wtf(f: FILETIME) -> u64 {
    (f.dwLowDateTime + (2 << 31) * f.dwHighDateTime) as u64
}

pub fn get_mem_stats(kind: &StatType) -> Result<PROCESS_MEMORY_COUNTERS, SporkError> {
    let handle = match kind {
        &StatType::Process => get_current_process(),
        &StatType::Thread => get_thread_handle(),
        &StatType::Children => {
            return Err(SporkError::new(
                SporkErrorKind::Unimplemented,
                "Windows child thread memory stat not yet implemented!".to_owned(),
            ))
        }
    };

    // SAFETY: Check the last windows error to ensure that the returned value
    // is in a valid state.
    let memory = unsafe {
        let mut memory = MaybeUninit::zeroed();
        let result = GetProcessMemoryInfo(
            handle,
            memory.as_mut_ptr(),
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );

        if result == 0 {
            let error = unsafe { GetLastError() };
            return Err(SporkError::new(
                SporkErrorKind::Unknown,
                format!("Win32Error: {error:x}"),
            ));
        }
        memory.assume_init()
    };
    Ok(memory)
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
            let mut lp_creation_time = MaybeUninit::zeroed();
            let mut lp_exit_time = MaybeUninit::zeroed();
            let mut lp_kernel_time = MaybeUninit::zeroed();
            let mut lp_user_time = MaybeUninit::zeroed();

            unsafe {
                let result = GetProcessTimes(
                    handle,
                    lp_creation_time.as_mut_ptr(),
                    lp_exit_time.as_mut_ptr(),
                    lp_kernel_time.as_mut_ptr(),
                    lp_user_time.as_mut_ptr(),
                );

                if result == 0 {
                    let error = unsafe { GetLastError() };
                    return Err(SporkError::new(
                        SporkErrorKind::Unknown,
                        format!("Win32Error: {error:x}"),
                    ));
                }
            };

            // SAFETY: The last windows error was checked to ensure all values requested
            // are in a valid state.
            let lp_creation_time = unsafe { lp_creation_time.assume_init() };
            let lp_exit_time = unsafe { lp_exit_time.assume_init() };
            let lp_kernel_time = unsafe { lp_kernel_time.assume_init() };
            let lp_user_time = unsafe { lp_user_time.assume_init() };

            Ok(WindowsCpuStats {
                creation: wtf(lp_creation_time),
                exit: wtf(lp_exit_time),
                kernel: wtf(lp_kernel_time),
                user: wtf(lp_user_time),
            })
        }
        StatType::Thread => {
            let handle = get_thread_handle();
            let mut lp_creation_time = MaybeUninit::zeroed();
            let mut lp_exit_time = MaybeUninit::zeroed();
            let mut lp_kernel_time = MaybeUninit::zeroed();
            let mut lp_user_time = MaybeUninit::zeroed();

            unsafe {
                let result = GetThreadTimes(
                    handle,
                    lp_creation_time.as_mut_ptr(),
                    lp_exit_time.as_mut_ptr(),
                    lp_kernel_time.as_mut_ptr(),
                    lp_user_time.as_mut_ptr(),
                );

                if result == 0 {
                    let error = unsafe { GetLastError() };
                    return Err(SporkError::new(
                        SporkErrorKind::Unknown,
                        format!("Win32Error: {error:x}"),
                    ));
                }
            };

            // SAFETY: The last windows error was checked to ensure all values requested
            // are in a valid state.
            let lp_creation_time = unsafe { lp_creation_time.assume_init() };
            let lp_exit_time = unsafe { lp_exit_time.assume_init() };
            let lp_kernel_time = unsafe { lp_kernel_time.assume_init() };
            let lp_user_time = unsafe { lp_user_time.assume_init() };

            Ok(WindowsCpuStats {
                creation: wtf(lp_creation_time),
                exit: wtf(lp_exit_time),
                kernel: wtf(lp_kernel_time),
                user: wtf(lp_user_time),
            })
        }
        StatType::Children => {
            return Err(SporkError::new(
                SporkErrorKind::Unimplemented,
                "Windows child thread memory stat not yet implemented!".to_owned(),
            ))
        }
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
