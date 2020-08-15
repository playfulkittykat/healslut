// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use async_std::fs::File;
use async_std::io::prelude::WriteExt;
use async_std::io::ReadExt;

use bit_vec::BitVec;

use directories::ProjectDirs;

use snafu::{IntoError, ResultExt, Snafu};

use std::io::ErrorKind;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Snafu)]
pub enum Error {
    NoDirectories,
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Deserialize {
        path: PathBuf,
        source: toml::de::Error,
    },
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, Eq,
)]
pub struct Config {
    pub target: Option<Target>,
}

impl Config {
    fn path() -> Result<PathBuf, Error> {
        let dirs = ProjectDirs::from(
            "rocks.tabby",
            "Tabby Rocks",
            env!("CARGO_PKG_NAME"),
        )
        .ok_or(Error::NoDirectories)?;

        let cfg = dirs.config_dir();

        std::fs::create_dir_all(cfg).with_context(|| CreateDir {
            path: cfg.to_owned(),
        })?;

        Ok(cfg.join("config.toml"))
    }

    pub async fn load() -> Result<Self, Error> {
        let path = Self::path()?;

        let mut file = match File::open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Ok(Default::default());
            }
            Err(e) => return Err(Open { path: path.clone() }.into_error(e)),
        };

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .with_context(|| Read { path: path.clone() })?;

        let parsed: Self = toml::from_slice(&contents)
            .with_context(|| Deserialize { path: path.clone() })?;

        Ok(parsed)
    }

    pub async fn save(&self) -> Result<(), Error> {
        let path = Self::path()?;

        let mut file = File::create(&path)
            .await
            .with_context(|| Open { path: path.clone() })?;

        let contents = toml::to_string_pretty(self).expect("serialize config");
        file.write_all(contents.as_bytes())
            .await
            .with_context(|| Write { path: path.clone() })?;

        Ok(())
    }
}

#[derive(
    Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Red,
    Green,
    Blue,

    Cyan,
    Magenta,
    Yellow,
    Black,
}

impl FromStr for Channel {
    type Err = ();

    fn from_str(txt: &str) -> Result<Self, Self::Err> {
        use self::Channel::*;

        let r = match txt {
            "Red" => Red,
            "Green" => Green,
            "Blue" => Blue,
            "Cyan" => Cyan,
            "Magenta" => Magenta,
            "Yellow" => Yellow,
            "Black" => Black,
            _ => return Err(()),
        };

        Ok(r)
    }
}

impl Channel {
    pub fn extract(self, red: u8, green: u8, blue: u8) -> u8 {
        match self {
            Channel::Red => return red,
            Channel::Green => return green,
            Channel::Blue => return blue,
            _ => (),
        }

        if red == 0 && green == 0 && blue == 0 {
            match self {
                Channel::Black => u8::max_value(),
                _ => 0,
            }
        } else {
            // TODO: Should be able to implement this without floating point.

            let r = red as f32 / u8::max_value() as f32;
            let g = green as f32 / u8::max_value() as f32;
            let b = blue as f32 / u8::max_value() as f32;

            let k = 1. - r.max(g).max(b);

            let mut float = match self {
                Channel::Cyan => (1. - r - k) / (1. - k),
                Channel::Magenta => (1. - g - k) / (1. - k),
                Channel::Yellow => (1. - b - k) / (1. - k),
                Channel::Black => k,
                _ => unreachable!(),
            };

            float *= u8::max_value() as f32;

            float.round() as u8
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TargetSpec {
    pub x: usize,
    pub y: usize,
    pub size: usize,

    pub channel: Channel,
    pub threshold: u8,
    pub count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Target {
    #[serde(flatten)]
    pub spec: TargetSpec,
    pub mask: BitVec,
}
