FROM rustembedded/cross:aarch64-unknown-linux-gnu-0.2.1 
# also hosted on: https://github.com/rust-embedded/cross/blob/master/docker/Dockerfile.armv7-unknown-linux-gnueabihf

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes libasound2-dev:arm64 && \
	apt-get install --assume-yes libx11-dev:arm64 && \
	apt-get install --assume-yes python3

RUN apt-get install --assume-yes pkg-config

RUN apt-get install --assume-yes gcc-arm-linux-gnueabihf
RUN apt-get install --assume-yes g++-arm-linux-gnueabihf
RUN apt-get install --assume-yes libfontconfig1-dev:arm64
RUN apt-get install --assume-yes libfreetype6-dev:arm64


ENV PKG_CONFIG_PATH="/usr/lib/aarch64-linux-gnu/pkgconfig:${PKG_CONFIG_PATH}"
