extern crate chrono;
extern crate rand;
extern crate spork;

#[allow(unused_imports)]
use spork::{Platform, Spork, SporkError, SporkErrorKind, StatType, Stats};

use std::thread;
use std::time;

use self::rand::distributions::{IndependentSample, Range};

use chrono::UTC;

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

fn now_ms() -> i64 {
    let now = UTC::now();
    (now.timestamp() * 1000 + (now.timestamp_subsec_millis() as i64)) as i64
}

fn rand_in_range(l: u64, r: u64) -> u64 {
    let between = Range::new(l, r);
    let mut rng = rand::thread_rng();
    between.ind_sample(&mut rng)
}

#[test]
fn should_poll_no_cpu() {
    let spork = match Spork::new() {
        Ok(s) => s,
        Err(e) => panic!("Error creating spork! {:?}", e),
    };

    sleep_ms!(5000);
    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };

    assert!(stats.cpu < 2_f64);
}

#[test]
fn should_poll_full_cpu() {
    let spork = match Spork::new() {
        Ok(s) => s,
        Err(e) => panic!("Error creating spork! {:?}", e),
    };

    fib(43);
    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };

    assert!(stats.cpu > 95_f64);
}

#[test]
fn should_get_linux_process_stats_fib_25() {
    // intentionally introduce some delays to simulate some weird contention for the clocks among
    // testing threads in order to hopefully draw out any bugs scoping the results between threads
    let wait = rand_in_range(100, 400);
    let expected_cpu = 1_f64;

    let before = now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(25);

    let stats = match spork.stats(StatType::Process) {
        Ok(s) => s,
        Err(e) => panic!("Stats error {:?}", e),
    };
    let _final = now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, 1);
    assert_eq!(stats.kind, StatType::Process);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
}

#[test]
fn should_get_linux_thread_stats_fib_35() {
    let wait = rand_in_range(100, 400);
    let expected_cpu = 10_f64;

    let before = now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(35);

    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Stats error {:?}", e),
    };
    let _final = now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, 1);
    assert_eq!(stats.kind, StatType::Thread);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
}

#[test]
fn should_get_low_cpu_linux_thread_stats() {
    let wait = rand_in_range(4000, 6000);
    let expected_cpu = 1.5_f64;

    let before = now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);

    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Stats error {:?}", e),
    };
    let _final = now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu < expected_cpu);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, 1);
    assert_eq!(stats.kind, StatType::Thread);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
}

#[test]
fn should_get_linux_process_stats_with_cpus() {
    let wait = 1500;
    let expected_cpu = 5_f64;

    let before = now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(35);

    let stats = match spork.stats_with_cpus(StatType::Process, Some(spork.num_cores())) {
        Ok(s) => s,
        Err(e) => panic!("Stats error {:?}", e),
    };
    let _final = now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.memory > 0);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, spork.num_cores());
    assert_eq!(stats.kind, StatType::Process);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
}

#[test]
fn should_get_linux_thread_stats_with_cpus() {
    let wait = rand_in_range(100, 400);
    let expected_cpu = 5_f64;

    let before = now_ms() as u64;
    let spork = Spork::new().unwrap();

    sleep_ms!(wait);
    // kick the cpu a bit
    fib(35);

    let stats = match spork.stats_with_cpus(StatType::Thread, Some(spork.num_cores())) {
        Ok(s) => s,
        Err(e) => panic!("Stats error {:?}", e),
    };
    let _final = now_ms() as u64;

    println!("{:?}", stats);
    assert!(stats.cpu > expected_cpu);
    assert!(stats.duration >= wait);
    assert!(stats.duration <= _final - before);
    assert_eq!(stats.cores, spork.num_cores());
    assert_eq!(stats.kind, StatType::Thread);
    assert!(stats.uptime >= wait);
    assert!(stats.uptime <= _final - before);
    assert!(stats.polled <= _final as i64);
}

#[test]
fn should_always_have_increasing_cpu_times() {
    let wait = 1005;

    let spork = Spork::new().unwrap();

    sleep_ms!(wait);

    let mut prev_times = vec![];
    for _x in 0..10 {
        let stats = match spork.stats(StatType::Process) {
            Ok(s) => s,
            Err(e) => panic!("Stats error {:?}", e),
        };
        prev_times.push(stats.cpu_time);
    }

    let mut prev_time: f64 = 0_f64;
    for time in &prev_times {
        assert!(time >= &prev_time);
        prev_time = *time;
    }
}

#[test]
fn should_always_have_increasing_cpus_times() {
    let wait = 1005;

    let spork = Spork::new().unwrap();

    sleep_ms!(wait);

    let mut prev_times = vec![];
    for _x in 0..10 {
        let stats = match spork.stats_with_cpus(StatType::Process, Some(spork.num_cores())) {
            Ok(s) => s,
            Err(e) => panic!("Stats error {:?}", e),
        };
        prev_times.push(stats.cpu_time);
    }

    let mut prev_time: f64 = 0_f64;
    for time in &prev_times {
        assert!(time >= &prev_time);
        prev_time = *time;
    }
}

#[test]
fn should_poll_half_cpu() {}

#[test]
fn should_poll_half_cpu_repeatedly() {}

#[test]
fn should_poll_full_cpu_repeatedly() {}

#[test]
fn should_poll_no_cpu_repeatedly() {}

// this is pretty much impossible to test reliably, since the tests are spread across threads
// and the cpu usage at any point in time is very difficult to forecast
#[test]
fn should_poll_process_cpu_repeatedly() {}
