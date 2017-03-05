
use std;

use winapi;
use winapi::*;
use kernel32;
use psapi::PROCESS_MEMORY_COUNTERS;

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
    dwHighDateTime: 0
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
    PeakPagefileUsage: 0
  }
}

// convert the two 32 bit ints in a FILETIME a u64
fn wtf(f: winapn::minwindef::FILETIME) -> u64 {
  f.dwLowDateTime + 2.pow(32) * f.dwHighDateTime
}

pub fn get_mem_stats(kind: &StatType) -> Result<PROCESS_MEMORY_COUNTER, Error> {

  match *kind {
    StatType::Process => {
      let mut handle = get_current_process();
      let mut memory = empty_proc_mem_counter(); 
      let mut cb = std::mem::size_of::<PROCESS_MEMORY_COUNTER>() as u32;

      unsafe {
        kernel32::K32GetProcessMemoryInfo(&mut handle, &mut memory, &mut cb);
      };

      Ok(memory)
    },
    StatType::Thread => {
      let mut handle = get_thread_handle();
      let mut memory = empty_proc_mem_counter(); 
      let mut cb = std::mem::size_of::<PROCESS_MEMORY_COUNTER>() as u32;

      unsafe {
        kernel32::K32GetProcessMemoryInfo(&mut handle, &mut memory, &mut cb);
      };

      Ok(memory) 
    },
    StatType::Children => {
      Err(PidError::new(PidErrorKind::Unimplemented, "Windows child thread memory stat not yet implemented!".to_owned()))
    }
  }
}

pub struct WindowsCpuStats {
  creation: u64,
  exit: u64,
  kernel: u64,
  user: u64
}

pub fn get_cpu_times(kind: &StatType) -> Result<WindowsCpuStats, Error> {

  match *kind {
    StatType::Process => {
      let mut handle = get_current_process();
      let mut lpCreationTime = empty_filetime();
      let mut lpExitTime = empty_filetime();
      let mut lpKernelTime = empty_filetime();
      let mut lpUserTime = empty_filetime(); 

      unsafe {
        kernel32::GetProcessTimes(&mut handle, &mut lpCreationTime, 
          &mut lpExitTime, &mut lpKernelTime, &mut lpUserTime);
      };

      Ok(WindowsCpuStats {
        creation: wtf(lpCreationTime),
        exit: wtf(lpExitTime),
        kernel: wtf(lpKernelTime),
        user: wtf(lpUserTime)
      })
    },
    StatType::Thread => {
      let mut handle = get_thread_handle();
      let mut lpCreationTime = empty_filetime();
      let mut lpExitTime = empty_filetime();
      let mut lpKernelTime = empty_filetime();
      let mut lpUserTime = empty_filetime();

      unsafe {
        kernel32::GetThreadTimes(&mut handle, &mut lpCreationTime, 
          &mut lpExitTime, &mut lpKernelTime, &mut lpUserTime);
      };

      Ok(WindowsCpuStats {
        creation: wtf(lpCreationTime),
        exit: wtf(lpExitTime),
        kernel: wtf(lpKernelTime),
        user: wtf(lpUserTime)
      })
    },
    StatType::Children => {
      // TODO
      Err(PidError::new(PidErrorKind::Unimplemented, "Windows child thread CPU time stat is not yet implemented!".to_owned()))
    }
  }

}

pub fn get_cpu_percent(history: &History, val: &WindowsCpuStats) -> f64 {
  unimplemented!();

}

pub fn get_clock_ticks() -> Result<i64, Error> {
  unimplemented!();
}

