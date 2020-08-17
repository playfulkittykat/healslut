#!/bin/bash
set -euf -o pipefail

choco install python --version=3.8.5

powershell Install-WindowsFeature Net-Framework-Core
choco install wixtoolset --version=3.11.2

cargo install cargo-wix

##
# Install MSYS2
##
[[ ! -f C:/tools/msys64/msys2_shell.cmd ]] && rm -rf C:/tools/msys64
choco uninstall -y mingw
choco upgrade --no-progress -y msys2
export msys2='cmd //C RefreshEnv.cmd '
export msys2+='& set MSYS=winsymlinks:nativestrict '
export msys2+='& C:\\tools\\msys64\\msys2_shell.cmd -defterm -no-start'
export msys2+=" -msys2 -c "\"\$@"\" --"
$msys2 pacman --sync --noconfirm --needed git pkg-config
taskkill //IM gpg-agent.exe //F || true # https://travis-ci.community/t/4967

if [ -d "C:/gtk-build/gtk" ]; then
    exit;
fi

cd "$TRAVIS_BUILD_DIR/.."
git clone https://github.com/wingtk/gvsbuild
cd gvsbuild

export PATH="/C/Windows/system32:/C/Windows/System32:/C/Windows/System32/Wbem:/C/Windows/System32/WindowsPowerShell/v1.0/:/C/tools/msys64/bin:/C/gtk-build/gtk/x64/release/bin"

$msys2 /C/Python38/python.exe build.py build -p x64 --vs-ver 15 --msys-dir "C:\\tools\\msys64" gtk3
