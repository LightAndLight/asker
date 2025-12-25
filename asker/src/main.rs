use std::{
    io::{BufRead, BufReader, Read},
    os::unix::net::UnixListener,
    path::PathBuf,
};

use uuid::Uuid;

struct Env {
    asker_dir: String,
}

fn read_env() -> Env {
    let asker_dir = std::env::var("ASKER_DIR").unwrap_or_else(|err| {
        match err {
            std::env::VarError::NotPresent => {
                eprintln!("error: ASKER_DIR not set");
            }
            std::env::VarError::NotUnicode(_) => {
                eprintln!("error: ASKER_DIR is not unicode");
            }
        }
        std::process::exit(1)
    });
    Env { asker_dir }
}

fn main() {
    unsafe {
        libc::umask(0o024);
    };

    let env = read_env();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("error: incorrect number of arguments");
        eprintln!("usage: {} KEY", args[0]);
        std::process::exit(1)
    }

    let key = args[1].as_str();

    let name = Uuid::new_v4();
    let socket_dir = PathBuf::from(env.asker_dir).join(key);
    match std::fs::exists(&socket_dir) {
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("error: no such key");
            std::process::exit(1)
        }
        Ok(false) => {
            eprintln!("error: no such key");
            std::process::exit(1)
        }
        Err(err) => {
            panic!("failed to locate {}: {err}", socket_dir.display())
        }
        _ => {}
    }

    let socket_path = socket_dir.join(name.to_string());
    {
        let listener = UnixListener::bind(&socket_path).unwrap_or_else(|err| {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!("error: not authorised");
                std::process::exit(1)
            } else {
                panic!("failed to bind to {}: {err}", socket_path.display())
            }
        });

        for connection in listener.incoming() {
            match connection {
                Err(err) => {
                    eprintln!("error: connection failed: {err}")
                }
                Ok(mut socket) => {
                    let mut value = String::new();
                    match socket.read_to_string(&mut value) {
                        Err(err) => {
                            eprintln!("error: failed to read from socket: {err}");
                        }
                        Ok(_count) => {}
                    }
                    println!("{value}");
                    break;
                }
            }
        }
    }

    std::fs::remove_file(&socket_path)
        .unwrap_or_else(|err| panic!("failed to remove {}: {err}", socket_path.display()));

    // remove garbage
    {
        let garbage_file_path = socket_dir.join("garbage");

        {
            let file = std::fs::File::open(&garbage_file_path).unwrap_or_else(|err| {
                panic!("failed to open {}: {err}", garbage_file_path.display())
            });

            for line in BufReader::new(file).lines() {
                let line = line.unwrap_or_else(|err| {
                    panic!(
                        "failed to read lines from {}: {err}",
                        garbage_file_path.display()
                    )
                });

                let entry_path = socket_dir.join(line);
                match std::fs::remove_file(&entry_path) {
                    Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                        panic!("failed to remove {}: {err}", entry_path.display())
                    }
                    _ => {}
                }
            }
        }

        let _file = std::fs::File::create(&garbage_file_path).unwrap_or_else(|err| {
            panic!("failed to truncate {}: {err}", garbage_file_path.display())
        });
    }
}
