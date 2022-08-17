use std::{
    thread::{self},
    time::Duration,
};

#[chrometracer::instrument(name = format!("{}", "hello"), tid = 1)]
fn hello() {
    println!("Hello");
}

fn main() {
    // let _guard = chrometracer::builder().init();

    let mut handles = vec![];
    for _ in 0..10 {
        handles.push(thread::spawn(|| {
            for _ in 0..10 {
                hello();
            }
        }));
    }

    std::thread::sleep(Duration::from_secs(1));
    handles
        .into_iter()
        .for_each(|handle| handle.join().unwrap());
}
