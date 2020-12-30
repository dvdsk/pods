#!/usr/bin/env bash

set -Eeuo pipefail # fail script if command fails

docker build -t pods/crosscompile:github - < Dockerfile
cross build --target=aarch64-unknown-linux-gnu

