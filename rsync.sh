#!/usr/bin/env bash

if [ -z "$1" ]; then
  echo "Usage: $0 <source>"
  exit 1
fi

rsync -avc -e ssh \
  --exclude .cargo \
  --exclude .embuild \
  --exclude .git \
  --exclude .github \
  --exclude .vscode \
  --exclude Cargo.lock \
  --exclude target \
  "$1" .
