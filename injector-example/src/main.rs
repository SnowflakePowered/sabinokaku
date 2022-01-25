use dll_syringe::{Syringe, Process};

fn main() {
    let target_process = Process::find_first_by_name("notepad").unwrap();
    let syringe = Syringe::new();
    syringe.inject(&target_process, r"D:\coding\sabitsuku\target\debug\sabinokaku_win.dll").unwrap();
}
