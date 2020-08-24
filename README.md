Healslut
========

## Introduction

Healslut is a desktop application that activates intimate hardware (eg. a vibrator) in response to what is displayed on your screen. For example, when Mercy, from Overwatch, is using her healing beam, the game displays a cross icon slightly below and to the left of the center of the screen. When that icon is displayed, healslut will turn the toy on!

### Disclaimer

**Healslut may be detected as a cheat or hack program. Use healslut at your own risk. You may get banned. You have been warned.**

That said, healslut does not perform any code injection, nor does it send any input to any games. On Windows, healslut uses the [Desktop Duplication API (DXGI)][0] to capture the screen. DXGI is one of the interfaces used by [OBS Studio][1].

[0]: https://docs.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
[1]: https://github.com/obsproject/obs-studio/blob/b31f04c92ad78daa455cbcb7f8d468ed99043b50/plugins/win-capture/graphics-hook/dxgi-capture.cpp

### Requirements

At minimum, healslut will require a supported toy and a Bluetooth adapter to connect to it. Most modern laptops have a Bluetooth adapter built-in, but many desktops don't. USB Bluetooth adapters are relatively commonplace, and should work with healslut.

### Supported Hardware

This tool is very new, and barely tested. The following hardware has been confirmed to at least somewhat work:

 * [Lovense Hush][hush]

Theoretically, however, any Bluetooth toy/device supported by the [Buttplug Sex Toy Control Project][bp] should work. Reports of functional (or not so functional) devices are welcome!

[hush]: https://www.lovense.com/vibrating-butt-plug
[bp]: https://buttplug.io/

## Installation

### Windows

Grab the latest MSI installer from the [Releases][3] page.

[3]: https://github.com/playfulkittykat/healslut/releases

### Ubuntu

No precompiled packages are provided yet.

## Configuration

### Picking a Target

Picking a target to trigger your device requires a bit of trial and error. The following steps work for Mercy from Overwatch, but should be adaptable to any game.

#### Taking a Screenshot

The first step is to take a screenshot of the game, in the state you want to target.

1. Open Overwatch, and enter the Practice Range.
2. Choose Mercy.
3. Engage your healing beam on a friendly robot.
4. Without disengaging the beam, press the Print Screen (`PrtScr`) button.
5. Exit Overwatch.

*Overwatch handles saving screenshots automatically. If your game does not, after pressing Print Screen, open Paint (included with Windows) and paste the image, then save it.*

#### Choosing the Target

After capturing a screenshot, you need to identify where on the screen healslut should focus.

1. Open healslut, and choose `Set Target...`.
2. Navigate to the screenshot and open it. Overwatch on Windows 10 saves screenshots in `C:\\Users\\YOURUSERNAME\\Documents\\Overwatch\\ScreenShots\\Overwatch`.
3. Click on the center of the healing icon as shown below.

![cross-cursor](https://user-images.githubusercontent.com/69809064/90995556-42f7b900-e58a-11ea-9cf1-dd12c0f187cf.png)

#### Isolating the Target

Clicking on the icon should open the specification window. The goal of these options is to isolate the icon (in grey/white) clearly on top of a black background.

 * **Side Length**: How large of a box should healslut pay attention to.
 * **Channel**: What component of the image is most important. If you don't see a preview, make sure to set this option to something. The channel determines what is the foreground, and what is the background.
 * **Threshold**: Channel value that distinguishes foreground from background. Adjust this to get a clear silhouette of the target symbol in white and grey against a black background.
 * **Tolerance**: How many mismatched pixels are allowed when looking for the symbol. Larger number are more forgiving, but may trigger more false positives.

The following values seem to work well for Mercy, but your mileage may vary:

![size=50 channel=green threshold=219 count=897](https://user-images.githubusercontent.com/69809064/90997290-29a53b80-e58f-11ea-8e2e-0c1bd3eabf93.png)

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
sudo setcap cap_net_raw+eip target/debug/healslut  # Required for bluetooth
```
