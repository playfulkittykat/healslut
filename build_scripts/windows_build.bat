call "C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

echo on

set PATH=%PATH%;%USERPROFILE%\.cargo\bin;C:\gtk-build\gtk\x64\release\bin;C:\tools\msys64\usr\bin;C:\Program Files (x86)\WiX Toolset v3.11\bin

cargo test --release || exit /b 1
cargo wix --nocapture || exit /b 1

mkdir target\gh || exit /b 1
move target\wix\*.msi target\gh\ || exit /b 1
