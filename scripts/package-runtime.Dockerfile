# SPDX-License-Identifier: EUPL-1.2
# Runtime QA only: no application checkout or Rust toolchain.
FROM ubuntu:24.04@sha256:52df9b1ee71626e0088f7d400d5c6b5f7bb916f8f0c82b474289a4ece6cf3faf
ENV DEBIAN_FRONTEND=noninteractive LIBGL_ALWAYS_SOFTWARE=1
RUN apt-get update && apt-get install -y \
    python3 xvfb libegl1 libgl1 libwayland-egl1 libxkbcommon0 \
    libxkbcommon-x11-0 libx11-6 libx11-xcb1 libxcursor1 libxi6 \
    && rm -rf /var/lib/apt/lists/*
