// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![allow(clippy::many_single_char_names)]

mod capture;
mod config;
mod exit;
mod gui;
mod plug;

use async_std::sync::Mutex;

use buttplug::client::device::ButtplugClientDevice;

use crate::exit::Exit;

use gio::prelude::*;

use std::env::args;
use std::sync::Arc;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Default)]
struct Shared {
    active: Option<ButtplugClientDevice>,
}

#[derive(Clone, Default)]
pub struct State {
    exit: Exit,
    shared: Arc<Mutex<Shared>>,
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let state = State::default();

    let application = gtk::Application::new(
        Some("rocks.tabby.healslut"),
        Default::default(),
    )?;

    let plug_state = state.clone();
    let (plug, plug_handle) = crate::plug::spawn(plug_state)?;
    let cap_handle = crate::capture::spawn(state.clone(), plug.clone())?;

    application.connect_activate(move |app| {
        crate::gui::Gui::build(app, plug.clone());
    });

    application.run(&args().collect::<Vec<_>>());

    plug::exit(&state);

    async_std::task::block_on(plug_handle);
    cap_handle.join().unwrap();

    Ok(())
}
