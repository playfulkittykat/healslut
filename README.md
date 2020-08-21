Healslut
========

## Introduction

Healslut is a desktop application that activates intimate hardware (eg. a vibrator) in response to what is displayed on your screen. For example, when Mercy, from Overwatch, is using her healing beam, the game displays a cross icon slightly below and to the left of the center of the screen. When that icon is displayed, healslut will turn the toy on!

### Disclaimer

**Healslut may be detected as a cheat or hack program. Use healslut at your own risk. You may get banned. You have been warned.**

That said, healslut does not perform any code injection, nor does it send any input to any games. On Windows, healslut uses the [Desktop Duplication API (DXGI)][0] to capture the screen. DXGI is one of the interfaces used by [OBS Studio][1].

[0]: https://docs.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
[1]: https://github.com/obsproject/obs-studio/blob/b31f04c92ad78daa455cbcb7f8d468ed99043b50/plugins/win-capture/graphics-hook/dxgi-capture.cpp

## Installation

### Windows

Grab the latest MSI installer from the [Releases][3] page.

[3]: https://github.com/playfulkittykat/healslut/releases

### Ubuntu

No precompiled packages are provided yet.

## Compiling from Source

### Windows

1. Install GTK3 from https://github.com/wingtk/gvsbuild
2. Install `pkg-config` into the MSYS2 from (1)
3. Then do the following in a Visual Studio Native Tools Command Prompt:

```
set PATH=%PATH%;C:\gtk-build\gtk\x64\release\bin;C:\msys64\usr\bin
cargo build
```

### Ubuntu

Install Rust, then:

```
sudo apt install libudev-dev libusb-1.0-0-dev libgtk-3-dev
cargo build
```
