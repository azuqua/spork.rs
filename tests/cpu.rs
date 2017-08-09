
extern crate spork;

#[allow(unused_imports)]
use spork::{
  SporkError,
  SporkErrorKind,
  Spork,
  StatType,
  Stats,
  Platform
};

use std::thread;
use std::time;

macro_rules! sleep_ms(
  ($($arg:tt)*) => { {
    thread::sleep(time::Duration::from_millis($($arg)*))
  } } 
);

fn fib(n: u64) -> u64 {
  if n > 2 {
    fib(n - 1) + fib(n - 2) 
  } else {
    1
  }
}

#[test]
fn should_poll_no_cpu() {
  let spork = match Spork::new() {
    Ok(s) => s,
    Err(e) => panic!("Error creating spork! {:?}", e)
  };

  sleep_ms!(5000);
  let stats = match spork.stats(StatType::Thread) {
    Ok(s) => s,
    Err(e) => panic!("Error polling stats! {:?}", e)
  };

  assert!(stats.cpu < 1_f64);
}

#[test]
fn should_poll_full_cpu() {
  let spork = match Spork::new() {
    Ok(s) => s,
    Err(e) => panic!("Error creating spork! {:?}", e)
  };

  fib(43);
  let stats = match spork.stats(StatType::Thread) {
    Ok(s) => s,
    Err(e) => panic!("Error polling stats! {:?}", e)
  };

  assert!(stats.cpu > 95_f64);
}

#[test]
fn should_poll_half_cpu() {


}

#[test]
fn should_poll_half_cpu_repeatedly() {


}

#[test]
fn should_poll_full_cpu_repeatedly() {


}

#[test]
fn should_poll_no_cpu_repeatedly() {


}

// this is pretty much impossible to test reliably, since the tests are spread across threads
// and the cpu usage at any point in time is very difficult to forecast
#[test]
fn should_poll_process_cpu_repeatedly() {
  

}
