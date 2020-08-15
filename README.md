Healslut
========

## Windows Build Notes

1. Install GTK3 from https://github.com/wingtk/gvsbuild
2. Install `pkg-config` into the MSYS2 from (1)
3. Then do the following in a Visual Studio Native Tools Command Prompt:

```
set PATH=%PATH%;C:\gtk-build\gtk\x64\release\bin;C:\msys64\usr\bin
cargo build
```
