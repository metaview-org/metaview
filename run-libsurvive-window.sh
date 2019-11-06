#!/usr/bin/env fish
rm -rf {,{HMD,WM{0,1}}_}config.json calinfo
set -gx XRT_COMPOSITOR_FORCE_XCB y
set -gx XR_RUNTIME_JSON $HOME/workspace/c/monado-libsurvive/build/openxr_monado-dev.json
set -gx RUST_BACKTRACE 1
cargo run $argv
