// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[cfg(windows)]
mod os {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn heat_dir<P: AsRef<Path>, S: Into<String>>(path: P, cg: S) {
        let cg = cg.into();

        let mut target_dir =
            PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        target_dir.push("wix");

        let out_file = target_dir.join(&format!("{}.wxs", cg));

        let mut heat_cmd = PathBuf::from(std::env::var_os("WIX").unwrap());
        heat_cmd.push("bin");
        heat_cmd.push("heat.exe");

        let output = Command::new(heat_cmd)
            .arg("dir")
            .arg(path.as_ref())
            .arg("-o")
            .arg(&out_file)
            .arg("-scom")
            .arg("-sfrag")
            .arg("-sreg")
            .arg("-cg")
            .arg(&cg)
            .arg("-dr")
            .arg("APPLICATIONFOLDER")
            .arg("-ag")
            .arg("-var")
            .arg(format!("var.{}Src", cg))
            .arg("-platform")
            .arg("x64")
            .spawn()
            .unwrap()
            .wait()
            .unwrap()
            .success();

        assert!(output, "Command Failed");
    }

    pub fn main() {
        heat_dir("C:\\gtk-build\\gtk\\x64\\release\\etc", "GtkEtc");
        heat_dir("C:\\gtk-build\\gtk\\x64\\release\\share", "GtkShare");
        heat_dir("C:\\gtk-build\\gtk\\x64\\release\\lib", "GtkLib");
    }
}

#[cfg(not(windows))]
mod os {
    pub fn main() {}
}

fn main() {
    self::os::main();
}
