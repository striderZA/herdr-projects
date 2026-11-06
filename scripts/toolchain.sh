#!/bin/sh
# Which Rust toolchain builds this repo here: on Windows the MSVC one needs Visual Studio's link.exe on the PATH; where
# there is none, the MinGW toolchain rustup has installed builds instead, unless you chose one yourself
# (RUSTUP_TOOLCHAIN). Sourced by scripts/setup.sh and scripts/check.sh; docs/testing.md.
tc_pick() {
  [ -z "$RUSTUP_TOOLCHAIN" ] || return 0
  case "$(uname -s 2>/dev/null)" in MINGW*|MSYS*) ;; *) return 0 ;; esac
  [ "$(rustc -vV 2>/dev/null | sed -n 's/^host: //p')" = "x86_64-pc-windows-msvc" ] || return 0
  # Git for Windows has a link.exe of its own (coreutils), which is no linker; MSVC's lives under Visual Studio.
  case "$(command -v link.exe 2>/dev/null)" in *"Microsoft Visual Studio"*) return 0 ;; esac
  rustup toolchain list 2>/dev/null | grep -q '^stable-x86_64-pc-windows-gnu' || return 0
  RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu
  export RUSTUP_TOOLCHAIN
  echo "No MSVC linker on PATH: building with the MinGW toolchain ($RUSTUP_TOOLCHAIN)." >&2
}
tc_pick
