#!/usr/bin/env fish
set -gx XR_RUNTIME_JSON $HOME/workspace/c/monado/build/openxr_monado-dev.json
set -gx RUST_BACKTRACE 1
cargo flamegraph $argv
