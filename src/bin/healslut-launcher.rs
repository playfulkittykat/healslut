// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(target_family = "windows", windows_subsystem = "windows")]

use directories::ProjectDirs;

use std::env;
use std::fs::{rename, File};
use std::process::Command;

fn open_log(name: &str) -> File {
    // TODO: Move this into a common library.
    let dirs =
        ProjectDirs::from("rocks.tabby", "Tabby Rocks", env!("CARGO_PKG_NAME"))
            .unwrap();

    let mut log_path = dirs.data_local_dir().join("logs");

    std::fs::create_dir_all(&log_path).expect("unable to create log directory");

    let mut old_path = log_path.clone();

    log_path.push(format!("healslut_{}.txt", name));
    old_path.push(format!("healslut_{}.old.txt", name));

    rename(&log_path, old_path).ok();

    File::create(log_path).expect("unable to open log file")
}

fn open_logs() -> (File, File) {
    (open_log("out"), open_log("err"))
}

#[cfg(target_family = "windows")]
fn main() {
    use std::path::PathBuf;

    let sysroot = PathBuf::from(
        env::var_os("SYSTEMROOT").expect("expected %SYSTEMROOT% to be set"),
    );
    let system32 = sysroot.join("SYSTEM32");

    let mut cur = env::current_exe().expect("cannot find path to current exe");
    cur.pop();

    let main = cur.join("healslut.exe");

    let path = env::join_paths(&[&sysroot, &system32, &cur])
        .expect("cannot join paths");

    let (out, err) = open_logs();

    Command::new(&main)
        .args(env::args_os().skip(1))
        .env_remove("PATH")
        .env("PATH", path)
        .stderr(err)
        .stdout(out)
        .spawn()
        .expect("unable to launch healslut")
        .wait()
        .unwrap();
}

#[cfg(target_family = "unix")]
fn main() {
    use std::os::unix::process::CommandExt;

    let (out, err) = open_logs();

    let mut cur = env::current_exe().expect("cannot find path to current exe");
    cur.pop();

    let main = cur.join("healslut");

    let error = Command::new(&main)
        .args(env::args_os().skip(1))
        .stdout(out)
        .stderr(err)
        .exec();

    eprintln!("unable to launch healslut: {}", error);
    std::process::exit(1);
}
