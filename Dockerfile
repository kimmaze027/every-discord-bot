# === Build stage ===
FROM rust:1.88-bookworm AS builder
WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build real app
COPY src/ src/
RUN touch src/main.rs && cargo build --release

# === Runtime stage ===
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg ca-certificates python3 pipx && \
    pipx install yt-dlp && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
ENV PATH="/root/.local/bin:$PATH"
COPY --from=builder /app/target/release/discord-music-bot /usr/local/bin/
CMD ["discord-music-bot"]
