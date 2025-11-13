# Development stage
FROM rust:1.88-slim AS development

WORKDIR /app

# Install development dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    cmake \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot reloading
RUN cargo install cargo-watch

# Copy workspace files
COPY backend/Cargo.toml backend/Cargo.lock ./backend/

# Copy individual package manifests
COPY backend/eldoria-server/Cargo.toml ./backend/eldoria-server/
COPY backend/encryption-service/Cargo.toml ./backend/encryption-service/
COPY backend/llm-providers/Cargo.toml ./backend/llm-providers/
COPY backend/storage-system/Cargo.toml ./backend/storage-system/

# Create dummy source files for dependency caching
RUN mkdir -p backend/eldoria-server/src \
    backend/encryption-service/src \
    backend/llm-providers/src \
    backend/storage-system/src && \
    echo "fn main() {}" > backend/eldoria-server/src/main.rs && \
    echo "pub fn dummy() {}" > backend/encryption-service/src/lib.rs && \
    echo "pub fn dummy() {}" > backend/llm-providers/src/lib.rs && \
    echo "pub fn dummy() {}" > backend/storage-system/src/lib.rs

# Build dependencies
RUN cargo build --release --manifest-path backend/Cargo.toml && \
    rm -rf backend/eldoria-server/src \
    backend/encryption-service/src \
    backend/llm-providers/src \
    backend/storage-system/src

# The actual source will be mounted as a volume in dev

# Builder stage
FROM rust:1.88-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the entire backend directory
COPY backend/ ./backend

# Build the application
RUN cargo build --release --manifest-path backend/Cargo.toml

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/backend/target/release/eldoria-server /app/eldoria-server

# Copy config files
COPY backend/eldoria-server/config ./config

# Create directory for static files
RUN mkdir -p /app/static

EXPOSE 8080

CMD ["./eldoria-server"]