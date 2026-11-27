# DimaSter in a container. The window needs the host display and GPU:
# start it with packaging/docker/run.sh. The shell runs inside the container.

FROM rust:1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libfontconfig1-dev libxkbcommon-dev libwayland-dev libx11-dev \
    libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p dimaster

FROM debian:bookworm-slim
# winit and wgpu load most of these at run time
RUN apt-get update && apt-get install -y --no-install-recommends \
    libfontconfig1 libxkbcommon0 libxkbcommon-x11-0 \
    libwayland-client0 libwayland-cursor0 libwayland-egl1 \
    libx11-6 libx11-xcb1 libxcursor1 libxi6 libxrandr2 libxcb1 \
    libvulkan1 mesa-vulkan-drivers libegl1 libgl1 libgl1-mesa-dri \
    fonts-dejavu-core fonts-noto-color-emoji \
    zsh fish bash less nano git curl ca-certificates ffmpeg procps pciutils locales \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/dimaster /usr/local/bin/dimaster
# any uid may be used at run time, so the home must be writable by everyone
RUN mkdir -p /home/dimaster && chmod 1777 /home/dimaster
ENV HOME=/home/dimaster SHELL=/usr/bin/zsh LANG=C.UTF-8
WORKDIR /home/dimaster
ENTRYPOINT ["/usr/local/bin/dimaster"]
