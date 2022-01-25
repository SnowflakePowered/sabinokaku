#[cfg(all(target_os = "windows"))]
use dll_syringe::{Syringe, Process};
use std::env::args;

#[cfg(all(target_os = "windows"))]
fn main() {
    let args: Vec<String> = args().collect();
    let target_process = Process::find_first_by_name(&args[1]).unwrap();
    let syringe = Syringe::new();
    syringe.inject(&target_process, &args[2]).unwrap();
}

#[cfg(not(target_os = "windows"))]
use std::io::Read;
#[cfg(not(target_os = "windows"))]
fn main() {
    println!("Hello World");
    let _input = std::io::stdin()
        .bytes()
        .next()
        .and_then(|result| result.ok());
}