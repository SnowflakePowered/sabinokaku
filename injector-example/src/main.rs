use dll_syringe::{Syringe, Process};
use std::io::Read;

fn main() {
    let target_process = Process::find_first_by_name("retroarch").unwrap();
    let syringe = Syringe::new();
    let injected_payload = syringe.inject(&target_process, r"D:\coding\sabitsuku\target\debug\sabitsuku_d3d.dll").unwrap();
    let _input = std::io::stdin()
        .bytes() 
        .next()
        .and_then(|result| result.ok());
    injected_payload.eject().unwrap();
}
