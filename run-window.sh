#!/usr/bin/env fish
set -gx XRT_COMPOSITOR_FORCE_XCB y
set -gx XR_RUNTIME_JSON $HOME/workspace/c/monado/build/openxr_monado-dev.json
set -gx RUST_BACKTRACE 1
cargo run $argv
