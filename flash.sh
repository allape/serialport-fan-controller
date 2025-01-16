#!/usr/bin/bash

features="esp-c3-32s"
flash_size="2mb"
if [ "$1" == "4mb" ]; then
  flash_size="4mb"
  features="esp32-c3-supermini"
fi

# cargo build --release
cargo espflash flash \
  --release \
  --no-default-features --features $features \
  --baud 921600 \
  --chip esp32c3 \
  --flash-size "$flash_size" \
  --partition-table "partitions_singleapp_large_$flash_size.csv" \
  --monitor
