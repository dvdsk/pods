#!/usr/bin/env bash

set -Eeuo pipefail # fail script if command fails

docker build --progress=plain -t pods/crosscompile:github .

cross build --target=aarch64-unknown-linux-gnu $1 --features "pinephone"
