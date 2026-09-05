use std::io::{Read, Write};
use std::path::Path;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetConsoleWindow() -> *mut std::ffi::c_void;
    fn GetStdHandle(which: u32) -> *mut std::ffi::c_void;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let folder = Path::new(&args[1]);
    // Bound failed probes too, including an accidentally blocking stdin read.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(6));
        std::process::exit(124);
    });
    std::fs::write(folder.join("phase"), b"before console check").unwrap();
    assert!(unsafe { GetConsoleWindow() }.is_null(), "visible console");
    for which in [-10_i32, -11, -12] {
        assert!(
            unsafe { GetStdHandle(which as u32) }.is_null(),
            "attached standard handle"
        );
    }
    std::fs::write(folder.join("phase"), b"before stdin read").unwrap();
    let mut input = Vec::new();
    assert_eq!(std::io::stdin().read_to_end(&mut input).unwrap(), 0);
    std::fs::write(folder.join("phase"), b"before stdout write").unwrap();
    std::io::stdout().write_all(b"discarded stdout").unwrap();
    std::io::stderr().write_all(b"discarded stderr").unwrap();
    println!("worker receipt");
    std::fs::write(folder.join("ready"), args[2..].join("\n")).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(4));
    std::fs::write(folder.join("done"), b"completed").unwrap();
}
