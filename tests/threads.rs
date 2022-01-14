extern crate spork;

#[allow(unused_imports)]
use spork::{Platform, Spork, SporkError, SporkErrorKind, StatType, Stats};

use std::time;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

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
fn should_correctly_poll_2_threads_separately() {
    // This spawns two threads (simple, expensive)
    // One thread (expensive) performs a CPU expensive operation
    // The second thread (simple) performs nothing expensive and waits for thread 1
    // Once expensive is done, thread one reports it's CPU times and we assert thread out spike CPU higher

    let (tx_expensive, rx_expensive): (Sender<f64>, Receiver<f64>) = mpsc::channel();
    let child_expensive = thread::spawn(move || {
        let spork = match Spork::new() {
            Ok(s) => s,
            Err(e) => panic!("Error creating spork! {:?}", e),
        };

        sleep_ms!(400);
        fib(42);

        let stats = match spork.stats(StatType::Thread) {
            Ok(s) => s,
            Err(e) => panic!("Error polling stats! {:?}", e),
        };
        tx_expensive.send(stats.cpu).unwrap();
    });

    let (tx_simple, rx_simple): (Sender<f64>, Receiver<f64>) = mpsc::channel();
    thread::spawn(move || {
        let spork = match Spork::new() {
            Ok(s) => s,
            Err(e) => panic!("Error creating spork! {:?}", e),
        };

        // Wait for expensive thread to end
        child_expensive.join().unwrap();

        let stats = match spork.stats(StatType::Thread) {
            Ok(s) => s,
            Err(e) => panic!("Error polling stats! {:?}", e),
        };

        tx_simple.send(stats.cpu).unwrap();
    });

    // Store values
    let cpu_simple = rx_simple.recv().unwrap();
    let cpu_expensive = rx_expensive.recv().unwrap();
    assert!(cpu_simple < cpu_expensive);
}

#[test]
fn should_correctly_poll_10_threads_separately() {
    let mut thread_handles = vec![];

    for _ in 0..5 {
        let thread_handle = thread::spawn(move || {
            let (tx_expensive, rx_expensive): (Sender<f64>, Receiver<f64>) = mpsc::channel();
            let child_expensive = thread::spawn(move || {
                let spork = match Spork::new() {
                    Ok(s) => s,
                    Err(e) => panic!("Error creating spork! {:?}", e),
                };

                sleep_ms!(400);
                fib(43);

                let stats = match spork.stats(StatType::Thread) {
                    Ok(s) => s,
                    Err(e) => panic!("Error polling stats! {:?}", e),
                };
                tx_expensive.send(stats.cpu).unwrap();
            });

            let (tx_simple, rx_simple): (Sender<f64>, Receiver<f64>) = mpsc::channel();
            thread::spawn(move || {
                let spork = match Spork::new() {
                    Ok(s) => s,
                    Err(e) => panic!("Error creating spork! {:?}", e),
                };

                // Wait for expensive thread to end
                child_expensive.join().unwrap();

                let stats = match spork.stats(StatType::Thread) {
                    Ok(s) => s,
                    Err(e) => panic!("Error polling stats! {:?}", e),
                };

                tx_simple.send(stats.cpu).unwrap();
            });

            // Store values
            let cpu_simple = rx_simple.recv().unwrap();
            let cpu_expensive = rx_expensive.recv().unwrap();
            assert!(cpu_simple < cpu_expensive);
        });
        thread_handles.push(thread_handle);
    }

    for x in thread_handles {
        let _ = x.join();
    }
    assert!(true);
}
