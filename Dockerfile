FROM ubuntu

# idk why but when using `apt update` (not `apt-get`) nvidia installation didn't work
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive apt install -y nvidia-driver-535
RUN apt install -y curl
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install tauri-cli trunk
RUN rustup target add wasm32-unknown-unknown
RUN apt update
RUN apt install -y \
    libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
