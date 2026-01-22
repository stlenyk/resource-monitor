#!/bin/bash

# `cargo clippy --workspace` doesn't work because it doesn't support multiple targets (see `.cargo/config.toml` files)

set -e

cargo clippy
cd src-tauri; cargo clippy
cd ../shared; cargo clippy
