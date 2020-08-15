// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use async_std::sync::{Mutex, Receiver, Sender};
use async_std::task::JoinHandle;

use bit_vec::{BitBlock, BitVec};

use buttplug::client::device::VibrateCommand;
use buttplug::client::{
    ButtplugClient, ButtplugClientError, ButtplugClientEvent,
};
use buttplug::connector::ButtplugInProcessClientConnector;
use buttplug::core::errors::{ButtplugDeviceError, ButtplugError};
use buttplug::server::comm_managers::btleplug::BtlePlugCommunicationManager;
use buttplug::util::async_manager;

use crate::config::{Config, Target, TargetSpec};
use crate::gui::Gui;
use crate::{Error, State};

use futures::stream::{Stream, StreamExt};

use gdk_pixbuf::Pixbuf;

use snafu::{ensure, OptionExt, Snafu};

use std::convert::TryFrom;
use std::fmt;
use std::marker::Unpin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Rect {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
}

pub struct Capture {
    pub width: usize,
    pub height: usize,
    pub rowstride: usize,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for Capture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Capture({}x{}, {} bytes)",
            self.width,
            self.height,
            self.bytes.len()
        )
    }
}

#[derive(Debug, Snafu)]
pub enum FromPixbufError {
    ChannelCount,
    Colorspace,
    NoAlpha,
}

impl TryFrom<Pixbuf> for Capture {
    type Error = FromPixbufError;

    fn try_from(mut p: Pixbuf) -> Result<Self, Self::Error> {
        ensure!(
            p.get_colorspace() == gdk_pixbuf::Colorspace::Rgb,
            Colorspace
        );

        if !p.get_has_alpha() {
            p = p.add_alpha(false, 0, 0, 0).context(NoAlpha)?;
        }

        ensure!(p.get_n_channels() == 4, ChannelCount);
        ensure!(p.get_has_alpha(), NoAlpha);

        Ok(Self {
            width: p.get_width() as usize,
            height: p.get_height() as usize,
            rowstride: p.get_rowstride() as usize,
            bytes: p.read_pixel_bytes().unwrap().to_vec(),
        })
    }
}

impl Capture {
    fn at(&self, x: usize, y: usize) -> Option<(u8, u8, u8, u8)> {
        let i = self.rowstride * y + 4 * x;
        let b = self.bytes.get(i)?;
        let g = self.bytes.get(i + 1)?;
        let r = self.bytes.get(i + 2)?;
        let a = self.bytes.get(i + 3)?;

        Some((*r, *g, *b, *a))
    }

    pub fn dims(&self, target: &TargetSpec) -> Rect {
        let half = target.size / 2;

        let x1 = std::cmp::min(target.x + half + 1, self.width);
        let x0 = if target.x < half { 0 } else { target.x - half };

        let y1 = std::cmp::min(target.y + half + 1, self.height);
        let y0 = if target.y < half { 0 } else { target.y - half };

        Rect { x0, x1, y0, y1 }
    }

    pub fn extract(&self, target: &TargetSpec) -> BitVec {
        let d = self.dims(target);
        let w = d.x1 - d.x0;
        let h = d.y1 - d.y0;

        let mut min = u8::max_value();
        let mut max = u8::min_value();

        for y in d.y0..d.y1 {
            for x in d.x0..d.x1 {
                let (r, g, b, _) = self.at(x, y).unwrap();
                let value = target.channel.extract(r, g, b);
                min = std::cmp::min(min, value);
                max = std::cmp::max(max, value);
            }
        }

        let mul = if max == min {
            1.
        } else {
            255. / (max - min) as f64
        };

        let mut buffer = BitVec::with_capacity(w * h);

        for y in d.y0..d.y1 {
            for x in d.x0..d.x1 {
                let (r, g, b, _) = self.at(x, y).unwrap();
                let value = target.channel.extract(r, g, b) - min;
                let rounded = (value as f64 * mul).round();
                let casted = if rounded > 255. {
                    u8::max_value()
                } else {
                    rounded as u8
                };

                buffer.push(casted >= target.threshold);
            }
        }

        buffer
    }

    fn check(&self, target: &Target) -> bool {
        let mut actual = self.extract(&target.spec);
        actual.xor(&target.mask);
        let distance: usize = actual.blocks().map(BitBlock::count_ones).sum();
        distance < target.spec.count
    }
}

#[derive(Debug)]
enum Msg {
    Scan,
    StopScan,
    Disconnect,
    SetTarget(Target),
    Capture(Capture),
}

#[derive(Debug, Clone)]
pub struct Plug(Sender<Msg>);

impl Plug {
    pub fn scan(&self) {
        async_std::task::block_on(self.0.send(Msg::Scan));
    }

    pub fn stop_scan(&self) {
        async_std::task::block_on(self.0.send(Msg::StopScan));
    }

    pub fn disconnect(&self) {
        async_std::task::block_on(self.0.send(Msg::Disconnect));
    }

    pub fn set_target(&self, target: Target) {
        async_std::task::block_on(self.0.send(Msg::SetTarget(target)));
    }

    pub fn capture(&self, capture: Capture) {
        async_std::task::block_on(self.0.send(Msg::Capture(capture)));
    }
}

pub fn exit(state: &State) {
    async_manager::block_on(async { state.exit.exit().await })
}

pub fn spawn(state: State) -> Result<(Plug, JoinHandle<()>), Error> {
    let (sender, receiver) = async_std::sync::channel(5);

    // Setup a client, and wait until everything is done before exiting.
    let handle = async_std::task::spawn(async move {
        let r = plug(state.clone(), receiver).await;
        state.exit.exit().await;
        if let Err(err) = r {
            eprintln!("{}", err);
            std::process::abort();
        }
    });

    Ok((Plug(sender), handle))
}

async fn plug(state: State, recv: Receiver<Msg>) -> Result<(), Error> {
    let config = Arc::new(Mutex::new(Config::load().await?));
    let vibing = Arc::new(AtomicBool::new(false));

    let connector = ButtplugInProcessClientConnector::new("Healslut Server", 0);
    let server = connector.server_ref();
    server.add_comm_manager::<BtlePlugCommunicationManager>()?;

    let (client, events) =
        ButtplugClient::connect("healslut", connector).await?;

    let res = async_std::task::spawn(handle_events(
        state.clone(),
        state.exit.from(events).await,
    ));

    let pattern_vibing = vibing.clone();
    let pattern_state = state.clone();
    let pattern = async_std::task::spawn(async move {
        let timer = async_std::stream::interval(Duration::from_millis(230));
        let mut timer = pattern_state.exit.from(timer).await;
        let mut on = false;

        while timer.next().await.is_some() {
            let shared = pattern_state.shared.lock().await;

            if let Some(ref active) = shared.active {
                if pattern_vibing.load(Ordering::SeqCst) {
                    if on {
                        active.vibrate(VibrateCommand::Speed(0.3)).await.ok();
                        on = false;
                    } else {
                        active.vibrate(VibrateCommand::Speed(0.8)).await.ok();
                        on = true;
                    }
                } else {
                    active.vibrate(VibrateCommand::Speed(0.0)).await.ok();
                    active.stop().await.ok();
                }
            }
        }
    });

    let from_ui = async_std::task::spawn(async move {
        let mut messages = state.exit.from(recv).await;

        while let Some(msg) = messages.next().await {
            eprintln!("UI -> Plug : {:?}", msg);

            match msg {
                Msg::StopScan => stop_scan(&client).await,
                Msg::Scan => scan(&state, &client).await,
                Msg::Disconnect => {
                    let mut shared = state.shared.lock().await;
                    if let Some(active) = shared.active.take() {
                        active.stop().await.ok();
                        Gui::send(|gui| gui.disconnected());
                        stop_scan(&client).await;
                    }
                }
                Msg::SetTarget(target) => {
                    let mut cfg = config.lock().await;
                    cfg.target = Some(target);
                    cfg.save().await.expect("unable to save config");
                }
                Msg::Capture(capture) => {
                    let cfg = config.lock().await;
                    let mut on = false;
                    if let Some(ref target) = cfg.target {
                        on = capture.check(target);
                    }
                    if on {
                        eprintln!("ON!");
                    }
                    vibing.store(on, Ordering::SeqCst);
                }
            }
        }
    });

    res.await;
    pattern.await;
    from_ui.await;

    Ok(())
}

async fn stop_scan(client: &ButtplugClient) {
    match client.stop_scanning().await {
        Ok(()) => (),
        Err(ButtplugClientError::ButtplugError(
            ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::DeviceScanningAlreadyStopped,
            ),
        )) => (),
        err => err.unwrap(),
    }

    Gui::send(move |gui| gui.stop_scan());
}

async fn scan(state: &State, client: &ButtplugClient) {
    let mut shared = state.shared.lock().await;

    if shared.active.is_some() {
        return;
    }

    if let Some(device) = client.devices().get(0) {
        shared.active = Some(device.clone());

        let name = device.name.clone();
        Gui::send(move |gui| {
            gui.connected(&name);
        });

        return;
    }

    match client.start_scanning().await {
        Ok(()) => (),
        Err(ButtplugClientError::ButtplugError(
            ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::DeviceScanningAlreadyStarted,
            ),
        )) => (),
        err => err.unwrap(),
    }

    Gui::send(move |gui| gui.start_scan());
}

async fn handle_events<S>(state: State, mut events: S)
where
    S: Stream<Item = ButtplugClientEvent> + Unpin,
{
    println!("Waiting for events");

    while let Some(event) = events.next().await {
        match event {
            ButtplugClientEvent::Error(e) => {
                eprintln!("Error: {}", e);
            }
            ButtplugClientEvent::DeviceAdded(device) => {
                eprintln!("Device Added: {}", device.name);

                let mut shared = state.shared.lock().await;
                if shared.active.is_none() {
                    let name = device.name.clone();
                    Gui::send(move |gui| {
                        gui.connected(&name);
                    });

                    shared.active = Some(device);
                }
            }
            ButtplugClientEvent::DeviceRemoved(device) => {
                eprintln!("Device Removed: {}", device.device_name);

                let mut shared = state.shared.lock().await;
                let devname = device.device_name.as_str();

                if shared.active.as_ref().map(|x| x.name.as_str())
                    == Some(devname)
                {
                    Gui::send(|gui| gui.disconnected());
                    shared.active = None;
                }
            }
            ButtplugClientEvent::Log(level, msg) => {
                eprintln!("LOG: {:?} {}", level, msg);
            }
            ButtplugClientEvent::ScanningFinished => {
                eprintln!("Scanning Finished");
                Gui::send(move |gui| gui.stop_scan());
            }
            ButtplugClientEvent::PingTimeout => {
                eprintln!("Ping Timeout");
            }
            ButtplugClientEvent::ServerDisconnect => {
                eprintln!("Server Disconnect");
            }
        }
    }

    println!("No more events");
    state.exit.exit().await;
}
