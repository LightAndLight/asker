#![feature(peer_credentials_unix_socket)]
use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fs::File,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Mutex,
};

use gtk4::{gdk::pango, gio, glib, prelude::*};
use inotify::Inotify;

const APP_ID: &str = "io.ielliott.asker-prompt";

const SPECIAL_ENTRIES: [&str; 2] = ["description", "garbage"];

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

fn request_input(socket_path: Rc<Path>) {
    let key_path: Rc<Path> = Rc::from(socket_path.parent().unwrap());

    let socket = match UnixStream::connect(&socket_path) {
        Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => {
            eprintln!("error: connection to {} refused", socket_path.display());
            let garbage_file_path = socket_path.parent().unwrap().join("garbage");
            let mut garbage_file = File::options()
                .append(true)
                .open(&garbage_file_path)
                .unwrap_or_else(|err| {
                    panic!("failed to open {}: {err}", garbage_file_path.display())
                });
            writeln!(
                garbage_file,
                "{}",
                socket_path.file_name().unwrap().display()
            )
            .unwrap_or_else(|err| {
                panic!("failed to write to {}: {err}", garbage_file_path.display())
            });

            return;
        }
        Err(err) => panic!("failed to connect to {}: {err}", socket_path.display()),
        Ok(socket) => Rc::new(Mutex::new(socket)),
    };

    let app = gtk4::Application::builder().application_id(APP_ID).build();
    app.connect_activate(glib::clone!(
        #[strong]
        key_path,
        #[strong]
        socket,
        move |app| on_activate(key_path.clone(), socket.clone(), app)
    ));
    app.set_accels_for_action("win.close", &["Escape"]);
    let _exit_code = app.run();
}

fn get_username(uid: u32) -> Option<String> {
    let result = unsafe { libc::getpwuid(uid) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(CString::from_raw((*result).pw_name).into_string().unwrap()) }
    }
}

fn main() -> glib::ExitCode {
    let env = Rc::new(read_env());

    let mut inotify = Inotify::init().expect("failed to init inotify");
    let asker_dir_items = std::fs::read_dir(&env.asker_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", env.asker_dir));

    let watch_descriptors: HashMap<inotify::WatchDescriptor, PathBuf> = asker_dir_items
        .map(|asker_dir_item| {
            let entry =
                asker_dir_item.unwrap_or_else(|err| panic!("failed to read directory item: {err}"));

            let entry_path = entry.path();

            let garbage: HashSet<String> = {
                let garbage_file_path = entry_path.join("garbage");
                let file = BufReader::new(std::fs::File::open(&garbage_file_path).unwrap_or_else(
                    |err| panic!("failed to open {}: {err}", garbage_file_path.display()),
                ));

                let mut lines = HashSet::new();
                for line in file.lines() {
                    let line = line.unwrap_or_else(|err| {
                        panic!(
                            "failed to read lines from {}: {err}",
                            garbage_file_path.display()
                        )
                    });
                    lines.insert(line);
                }
                lines
            };

            for entry in std::fs::read_dir(&entry_path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", entry_path.display()))
            {
                let entry =
                    entry.unwrap_or_else(|err| panic!("failed to read pending request: {err}"));

                let entry_file_name = entry.file_name();
                if !SPECIAL_ENTRIES.contains(&entry_file_name.to_str().unwrap())
                    && !garbage.contains(entry_file_name.to_str().unwrap())
                {
                    let socket_path = Rc::from(entry.path());
                    request_input(socket_path);
                }
            }
            let watch_descriptor = inotify
                .watches()
                .add(entry_path.clone(), inotify::WatchMask::CREATE)
                .unwrap_or_else(|err| panic!("failed to watch {}: {err}", entry_path.display()));
            (watch_descriptor, entry_path)
        })
        .collect();

    loop {
        let mut buffer = [0; 1024];
        let events = inotify
            .read_events_blocking(&mut buffer)
            .unwrap_or_else(|err| panic!("failed to read event: {err}"));

        for event in events {
            if event.mask == inotify::EventMask::CREATE
                && let Some(path) = watch_descriptors.get(&event.wd)
            {
                let socket_path = Rc::from(path.join(event.name.unwrap()));
                request_input(socket_path);
            }
        }
    }
}

fn on_activate(key_path: Rc<Path>, socket: Rc<Mutex<UnixStream>>, app: &gtk4::Application) {
    let peer_cred = {
        let socket = socket.lock().unwrap();
        match socket.peer_cred() {
            Err(err) => {
                eprintln!("error: socket peer credentials failed: {err}");
                None
            }
            Ok(peer_cred) => Some(peer_cred),
        }
    };

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("asker-prompt")
        .build();

    // https://github.com/gtk-rs/gtk/issues/949#issuecomment-581618386
    let font_size_pixels = match window.pango_context().font_description() {
        None => 16,
        Some(font_description) => font_description.size() / pango::SCALE,
    };

    let message_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .hexpand(false)
        .build();

    let peer_username: Option<String> =
        { peer_cred.and_then(|peer_cred| get_username(peer_cred.uid)) };

    let key_description: Option<String> = {
        let key_description_path: PathBuf = key_path.join("description");
        match File::open(&key_description_path) {
            Err(err) => {
                eprintln!(
                    "error: failed to open {}: {err}",
                    key_description_path.display()
                );
                None
            }
            Ok(file) => BufReader::new(file)
                .lines()
                .next()
                .and_then(|line| match line {
                    Err(err) => {
                        eprintln!(
                            "error: failed to read {}: {err}",
                            key_description_path.display()
                        );
                        None
                    }
                    Ok(value) => Some(value),
                }),
        }
    };
    let message = format!(
        "{} is requesting {}",
        peer_username.as_deref().unwrap_or("unknown user"),
        key_description.as_deref().unwrap_or("unknown key")
    );
    let message_label = gtk4::Label::new(Some(&message));
    message_box.append(&message_label);

    let inputs_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .vexpand(false)
        .build();

    let password_entry = gtk4::PasswordEntry::builder().max_width_chars(24).build();
    inputs_box.append(&password_entry);

    let submit_button = gtk4::Button::builder()
        .label("Submit")
        .margin_start(font_size_pixels)
        .build();
    inputs_box.append(&submit_button);

    let main_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(false)
        .valign(gtk4::Align::Center)
        .halign(gtk4::Align::Center)
        .build();
    main_box.append(&message_box);
    main_box.append(&inputs_box);

    window.set_child(Some(&main_box));
    window.add_action_entries([gio::ActionEntryBuilder::new("close")
        .activate(|window: &gtk4::ApplicationWindow, _, _| {
            window.close();
        })
        .build()]);

    password_entry.connect_activate(glib::clone!(
        #[strong]
        socket,
        #[weak]
        window,
        #[weak]
        password_entry,
        move |_this| action_generate_and_copy(socket.clone(), window, password_entry)
    ));
    submit_button.connect_clicked(glib::clone!(
        #[strong]
        socket,
        #[weak]
        window,
        #[weak]
        password_entry,
        move |_this| action_generate_and_copy(socket.clone(), window, password_entry)
    ));

    window.present();
}

fn action_generate_and_copy(
    socket: Rc<Mutex<UnixStream>>,
    window: gtk4::ApplicationWindow,
    password_entry: gtk4::PasswordEntry,
) {
    {
        let mut socket = socket.lock().unwrap();
        socket
            .write_all(password_entry.text().as_bytes())
            .unwrap_or_else(|err| panic!("failed to write to socket: {err}"));
    }

    window.close()
}
