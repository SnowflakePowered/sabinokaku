#[cfg(all(target_os = "windows"))]
use dll_syringe::{Syringe, Process};

#[cfg(all(target_os = "windows"))]
fn main() {
    let target_process = Process::find_first_by_name("notepad").unwrap();
    let syringe = Syringe::new();
    syringe.inject(&target_process, r"D:\coding\sabitsuku\target\debug\sabinokaku_win.dll").unwrap();
}

#[cfg(all(target_os = "linux"))]
use std::io::Read;
#[cfg(all(target_os = "linux"))]
fn main() {
    println!("Hello World");
    let _input = std::io::stdin()
        .bytes()
        .next()
        .and_then(|result| result.ok());
}