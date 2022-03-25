FROM rust:latest

RUN apt-get update && apt-get install -y \ 
    g++-arm-linux-gnueabihf lib32stdc++6 libc6-dev-armhf-cross libclang-dev cmake \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add armv7-unknown-linux-gnueabihf
RUN cargo install cargo-deb

WORKDIR /app

ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
    CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc \
    CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++
ENV BINDGEN_EXTRA_CLANG_ARGS='--sysroot=/usr/arm-linux-gnueabihf/ \
	-I/usr/arm-linux-gnueabihf/include/c++/10/ \
	-I/usr/arm-linux-gnueabihf/include/c++/10/arm-linux-gnueabihf/'

CMD ["cargo", "deb", "--target", "armv7-unknown-linux-gnueabihf"]
