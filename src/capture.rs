// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::plug::{Capture, Plug};
use crate::{Error, State};

use scrap::{Capturer, Display};

use std::io::ErrorKind;
use std::thread::JoinHandle;
use std::time::Duration;

pub fn spawn(state: State, plug: Plug) -> Result<JoinHandle<()>, Error> {
    let handle =
        std::thread::Builder::new()
            .name("scrcap".into())
            .spawn(move || {
                let mut opt_capturer = None;

                let mut interval =
                    state.exit.interval(Duration::from_millis(1000));

                while interval.next().is_some() {
                    let capturer = opt_capturer.get_or_insert_with(|| {
                        let display = Display::primary().unwrap();
                        Capturer::new(display).unwrap()
                    });

                    let w = capturer.width();
                    let h = capturer.height();

                    let buffer = match capturer.frame() {
                        Ok(buffer) => buffer,
                        Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
                        Err(other) => {
                            eprintln!("Screen capture error: {}", other);
                            opt_capturer = None;
                            continue;
                        }
                    };

                    let bytes = buffer.to_owned();
                    let capture = Capture {
                        rowstride: bytes.len() / h,
                        height: h,
                        width: w,
                        bytes,
                    };

                    plug.capture(capture);
                }
            })?;

    Ok(handle)
}
