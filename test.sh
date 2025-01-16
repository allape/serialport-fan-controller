#!/usr/bin/bash

cargo build --release --no-default-features --features esp-c3-32s
cargo build --release --no-default-features --features esp32-c3-supermini
