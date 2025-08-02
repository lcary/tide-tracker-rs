FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main

RUN apt-get update && apt-get install -y --no-install-recommends \
  git curl zip dosfstools e2fsprogs genimage mtools \
  mmdebstrap bdebstrap podman crudini zstd pv uidmap \
  python-is-python3 qemu-user-static rsync coreutils && \
  rm -rf /var/lib/apt/lists/*

RUN git clone --depth=1 https://github.com/raspberrypi/rpi-image-gen.git /opt/rpi-image-gen
WORKDIR /opt/rpi-image-gen

ENTRYPOINT ["/bin/bash"]
