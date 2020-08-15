// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::env;
use std::process::Command;

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

    Command::new(&main)
        .args(env::args_os().skip(1))
        .env_remove("PATH")
        .env("PATH", path)
        .spawn()
        .expect("unable to launch healslut")
        .wait()
        .unwrap();
}

#[cfg(target_family = "unix")]
fn main() {
    use std::os::unix::process::CommandExt;

    let mut cur = env::current_exe().expect("cannot find path to current exe");
    cur.pop();

    let main = cur.join("healslut");

    let error = Command::new(&main).args(env::args_os().skip(1)).exec();

    eprintln!("unable to launch healslut: {}", error);
    std::process::exit(1);
}
