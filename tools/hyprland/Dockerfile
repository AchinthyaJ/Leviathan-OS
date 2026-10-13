# Builds Hyprland 0.56.2 and its pinned Hyprland-org dependencies on Ubuntu 26.04.
# Hyprland needs C++26, which Ubuntu 24.04's GCC 13 cannot build; 25.10 ships GCC 15.
#
# Export the install prefix (/opt/hypr) into build/hyprland-sysroot/:
#   docker build --target out -o build/hyprland-sysroot tools/hyprland

FROM ubuntu:26.04 AS build
SHELL ["/bin/bash", "-c"]
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates git curl build-essential g++-16 gcc-16 \
        cmake ninja-build meson pkgconf python3 \
        libwayland-dev wayland-protocols libxkbcommon-dev uuid-dev \
        libcairo2-dev libpango1.0-dev libpixman-1-dev libxcursor-dev \
        libdrm-dev libinput-dev libeis-dev libgbm-dev libgio-2.0-dev \
        libre2-dev libmuparser-dev liblcms2-dev glslang-dev glslang-tools \
        libudis86-dev libseat-dev libdisplay-info-dev hwdata \
        libxcb-errors-dev libxcb-res0-dev libxcb-icccm4-dev libxcb-composite0-dev \
        libxcb-render0-dev libxcb-xfixes0-dev libxcb-dri3-dev libxcb-present-dev \
        libxcb-xkb-dev libxcb-shm0-dev libxcb-util-dev libxcb-ewmh-dev \
        libxcb-image0-dev libxcb-keysyms1-dev libxcb-randr0-dev libxcb-xinput-dev \
        libliftoff-dev libtomlplusplus-dev libgtest-dev libzip-dev \
        libgles-dev libegl-dev libopengl-dev libglvnd-dev \
        libpugixml-dev librsvg2-dev libmagic-dev libjpeg-dev libwebp-dev libpng-dev \
        libudev-dev libsystemd-dev libffi-dev libreadline-dev \
    && rm -rf /var/lib/apt/lists/*

ENV CC=gcc-16 CXX=g++-16 PREFIX=/opt/hypr
ENV PKG_CONFIG_PATH=/opt/hypr/lib/pkgconfig:/opt/hypr/share/pkgconfig
ENV CMAKE_PREFIX_PATH=/opt/hypr PATH=/opt/hypr/bin:$PATH
WORKDIR /src

# Pinned revisions from Hyprland v0.56.2's flake.lock.
# Each source is fetched at its exact commit, so the build does not drift.
ARG HYPRUTILS_REV=5a7b8cf221914ce4714407950e4ffbdddcd8b66f
ARG HYPRWAYLAND_SCANNER_REV=b8632713a6beaf28b56f2a7b0ab2fb7088dbb404
ARG HYPRLANG_REV=090117506ddc3d7f26e650ff344d378c2ec329cc
ARG HYPRGRAPHICS_REV=8699c38f0e4a1ca3bfc84f84ba020509ced1f133
ARG HYPRCURSOR_REV=39435900785d0c560c6ae8777d29f28617d031ef
ARG AQUAMARINE_REV=1a10fe26a9f7d989c359e6a9ea61aa2e44d06c36
ARG HYPRLAND_PROTOCOLS_REV=1cb6db5fd6bb8aee419f4457402fa18293ace917
ARG HYPRWIRE_REV=7d935bb54674aa0fbd327d2a6888bd0630079ed0
ARG HYPRLAND_TAG=v0.56.2

# Helper: clone one commit shallowly.
RUN cat > /usr/local/bin/fetch-rev <<'EOF' && chmod +x /usr/local/bin/fetch-rev
#!/bin/bash
# usage: fetch-rev <repo-url> <sha> <dir>
set -euo pipefail
git init -q "$3" && cd "$3" && git remote add origin "$1" \
  && git fetch -q --depth 1 origin "$2" && git checkout -q FETCH_HEAD
EOF

# Lua 5.5 (Hyprland requires lua55/lua5.5 via pkg-config; not packaged on 25.10).
RUN curl -fsSL https://www.lua.org/ftp/lua-5.5.0.tar.gz | tar xz \
 && cd lua-5.5.0 && make -j"$(nproc)" linux MYCFLAGS="-fPIC" \
 && make install INSTALL_TOP=/opt/hypr \
 && mkdir -p /opt/hypr/lib/pkgconfig \
 && printf 'prefix=/opt/hypr\nlibdir=${prefix}/lib\nincludedir=${prefix}/include\n\nName: Lua\nDescription: Lua 5.5\nVersion: 5.5.0\nLibs: -L${libdir} -llua -lm -ldl\nCflags: -I${includedir}\n' > /opt/hypr/lib/pkgconfig/lua55.pc \
 && cp /opt/hypr/lib/pkgconfig/lua55.pc /opt/hypr/lib/pkgconfig/lua5.5.pc

# Hyprland-org libraries, in dependency order.
RUN fetch-rev https://github.com/hyprwm/hyprutils $HYPRUTILS_REV hyprutils \
 && cmake -S hyprutils -B hyprutils/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX -DBUILD_TESTING=OFF \
 && cmake --build hyprutils/b && cmake --install hyprutils/b

RUN fetch-rev https://github.com/hyprwm/hyprwayland-scanner $HYPRWAYLAND_SCANNER_REV hyprwayland-scanner \
 && cmake -S hyprwayland-scanner -B hyprwayland-scanner/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX \
 && cmake --build hyprwayland-scanner/b && cmake --install hyprwayland-scanner/b

RUN fetch-rev https://github.com/hyprwm/hyprlang $HYPRLANG_REV hyprlang \
 && cmake -S hyprlang -B hyprlang/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX -DBUILD_TESTING=OFF \
 && cmake --build hyprlang/b && cmake --install hyprlang/b

RUN fetch-rev https://github.com/hyprwm/hyprgraphics $HYPRGRAPHICS_REV hyprgraphics \
 && cmake -S hyprgraphics -B hyprgraphics/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX -DBUILD_TESTING=OFF \
 && cmake --build hyprgraphics/b && cmake --install hyprgraphics/b

RUN fetch-rev https://github.com/hyprwm/hyprcursor $HYPRCURSOR_REV hyprcursor \
 && cmake -S hyprcursor -B hyprcursor/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX -DBUILD_TESTING=OFF \
 && cmake --build hyprcursor/b && cmake --install hyprcursor/b

RUN fetch-rev https://github.com/hyprwm/hyprland-protocols $HYPRLAND_PROTOCOLS_REV hyprland-protocols \
 && cmake -S hyprland-protocols -B hyprland-protocols/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX \
 && cmake --build hyprland-protocols/b && cmake --install hyprland-protocols/b

RUN fetch-rev https://github.com/hyprwm/aquamarine $AQUAMARINE_REV aquamarine \
 && cmake -S aquamarine -B aquamarine/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX -DBUILD_TESTING=OFF \
 && cmake --build aquamarine/b && cmake --install aquamarine/b

# hyprwire (IPC library and scanner that hyprctl needs).
RUN fetch-rev https://github.com/hyprwm/hyprwire $HYPRWIRE_REV hyprwire \
 && cmake -S hyprwire -B hyprwire/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX -DBUILD_TESTING=OFF \
 && cmake --build hyprwire/b && cmake --install hyprwire/b

# wayland-protocols 1.49 (Hyprland requires >= 1.49; Ubuntu 26.04 ships 1.47).
# Ubuntu's wayland-scanner (1.24) rejects 1.49's XML under --strict (DTD warning), so drop that flag here.
RUN git clone -q --depth 1 --branch 1.49 https://gitlab.freedesktop.org/wayland/wayland-protocols /src/wayland-protocols \
 && sed -i "/^\s*'--strict',\s*$/d" /src/wayland-protocols/include/wayland-protocols/meson.build \
 && meson setup /src/wayland-protocols/b /src/wayland-protocols --prefix=$PREFIX -Dtests=false \
 && meson install -C /src/wayland-protocols/b

# Hyprland itself. Upstream tag, not a pinned SHA, because it is the release under test.
RUN git clone -q --depth 1 --branch $HYPRLAND_TAG --recursive https://github.com/hyprwm/Hyprland /src/Hyprland \
 && cmake -S Hyprland -B Hyprland/b -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$PREFIX \
        -DNO_SYSTEMD=ON -DNO_UWSM=ON -DNO_HYPRPM=ON \
 && cmake --build Hyprland/b && cmake --install Hyprland/b

# Hyprland links udis86 dynamically; bundle the Ubuntu shared lib so the sysroot is self-contained.
RUN cp -a /usr/lib/x86_64-linux-gnu/libudis86.so* /opt/hypr/lib/

# Export only the install prefix and the Lua/Hyprland build metadata.
FROM scratch AS out
COPY --from=build /opt/hypr /opt/hypr
