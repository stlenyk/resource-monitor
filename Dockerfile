FROM rust

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
