FROM rustembedded/cross:aarch64-unknown-linux-gnu-0.2.1 
# also hosted on: https://github.com/rust-embedded/cross/blob/master/docker/Dockerfile.armv7-unknown-linux-gnueabihf

# not expecting a cache mechanism like the one in buildx, the base image includes
# this config file that essentially purges the cache on every install operation
# which is why we have to remove this config to take advantage of the host's cache
RUN rm /etc/apt/apt.conf.d/docker-clean

RUN \
    # holds the package _indexes_
    --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    # holds the package _contents_
    --mount=type=cache,target=/var/cache/apt/archives,sharing=locked \
    dpkg --add-architecture arm64 && \
    apt update
RUN \
    # holds the package _indexes_
    --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    # holds the package _contents_
    --mount=type=cache,target=/var/cache/apt/archives,sharing=locked \
    apt install --assume-yes \
        libasound2-dev:arm64 \
        libx11-dev:arm64 \
        python3 \
        pkg-config \
        gcc-arm-linux-gnueabihf \
        g++-arm-linux-gnueabihf \
        libfontconfig1-dev:arm64 \
        libfreetype6-dev:arm64 \
        clang \
        build-essential


ENV PKG_CONFIG_PATH="/usr/lib/aarch64-linux-gnu/pkgconfig:${PKG_CONFIG_PATH}"
