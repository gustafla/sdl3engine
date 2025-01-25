#!/bin/bash
set -e

if [[ -z "$1" ]]; then echo "Usage: $0 <ARCH>"; exit 1; fi
ARCH=$1
PKG=$(basename "$PWD")

podman build --arch ${ARCH} --tag "${PKG}-${ARCH}" .
podman run \
  --arch ${ARCH} \
  --rm \
  --userns keep-id \
  -v "$PWD":"/usr/src/$PKG" \
  -w "/usr/src/$PKG" \
  "${PKG}-${ARCH}" \
  cargo build --target-dir "target/${ARCH}" --locked --release
