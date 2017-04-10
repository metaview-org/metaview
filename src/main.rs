use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;

pub const NMAP_NAME: &'static str = "nmap";

fn main() {
    let mut command = Command::new(NMAP_NAME);

    command.arg("-oX");
    command.arg("-");

    for arg in std::env::args() {
        command.arg(&arg);
    }

    command.spawn()
        .unwrap()
        .wait();
}
