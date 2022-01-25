#[cfg(all(target_os = "windows"))]
fn main() {
    use dll_syringe::{Syringe, Process};
    use std::env::args;

    let args: Vec<String> = args().collect();
    let target_process = Process::find_first_by_name(&args[1]).unwrap();
    let syringe = Syringe::new();
    syringe.inject(&target_process, &args[2]).unwrap();
}

#[cfg(not(target_os = "windows"))]
use std::io::Read;
#[cfg(not(target_os = "windows"))]
fn main() {
    use std::env::vars;
    println!("Hello World");
    for (k, v) in vars() {
        println!("{} {}", k, v);
    }
    let _input = std::io::stdin()
        .bytes()
        .next()
        .and_then(|result| result.ok());
}