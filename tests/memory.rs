extern crate spork;

#[allow(unused_imports)]
use spork::{Platform, Spork, SporkError, SporkErrorKind, StatType, Stats};

#[test]
fn should_poll_no_memory_change_process() {
    let spork = match Spork::new() {
        Ok(s) => s,
        Err(e) => panic!("Error creating spork! {:?}", e),
    };

    let stats = match spork.stats(StatType::Process) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let start_memo = stats.memory;

    let stats = match spork.stats(StatType::Process) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let end_memo = stats.memory;

    assert!(start_memo == end_memo);
}

#[test]
fn should_poll_increased_memory_process() {
    let spork = match Spork::new() {
        Ok(s) => s,
        Err(e) => panic!("Error creating spork! {:?}", e),
    };

    let mut n = 0;
    let mut v: Vec<i32> = vec![];

    let stats = match spork.stats(StatType::Process) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let start_memo = stats.memory;

    while n < 1000000 {
        v.push(255);
        n = n + 1;
    }

    let stats = match spork.stats(StatType::Process) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let end_memo = stats.memory;

    assert!(start_memo < end_memo);
}

#[test]
fn should_poll_no_memory_change_thread() {
    let spork = match Spork::new() {
        Ok(s) => s,
        Err(e) => panic!("Error creating spork! {:?}", e),
    };

    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let start_memo = stats.memory;

    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let end_memo = stats.memory;

    assert!(start_memo == end_memo);
}

#[test]
fn should_poll_increased_memory_thread() {
    let spork = match Spork::new() {
        Ok(s) => s,
        Err(e) => panic!("Error creating spork! {:?}", e),
    };

    let mut n = 0;
    let mut v: Vec<i32> = vec![];

    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let start_memo = stats.memory;
    println!("STATS");
    println!("{:?}", stats);

    while n < 1000000 {
        v.push(255);
        n = n + 1;
    }

    let stats = match spork.stats(StatType::Thread) {
        Ok(s) => s,
        Err(e) => panic!("Error polling stats! {:?}", e),
    };
    let end_memo = stats.memory;

    println!("STATS");
    println!("{:?}", stats);

    assert!(start_memo < end_memo);
}
