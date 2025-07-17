# Development stage
FROM rust:1.87-slim AS development

WORKDIR /app

# Install development dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot reloading
RUN cargo install cargo-watch

# bring in the wit definitions for bindgen
COPY plugin-api/wit ./plugin-api/wit
COPY plugin-api/Cargo.toml ./plugin-api/

# Copy workspace files
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/server/Cargo.toml ./backend/server/
COPY backend/llm-providers/Cargo.toml ./backend/llm-providers/

# Create dummy source files for dependency caching
RUN mkdir -p backend/server/src backend/llm-providers/src plugin-api/src && \
    echo "fn main() {}" > backend/server/src/main.rs && \
    echo "pub fn dummy() {}" > backend/llm-providers/src/lib.rs && \
    echo "pub fn dummy() {}" > plugin-api/src/lib.rs

# Build dependencies
RUN cargo build --release --manifest-path plugin-api/Cargo.toml && \
    cargo build --release --manifest-path backend/Cargo.toml && \
    rm -rf backend/server/src backend/llm-providers/src

# The actual source will be mounted as a volume in dev

# Builder stage
FROM rust:1.87-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the entire backend directory
COPY backend/ ./backend

# Copy plugin-api so bindgen can see your .wit files
COPY plugin-api ./plugin-api

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/backend/target/release/server /app/eldoria-hub/backend
COPY --from=builder /app/plugin-api/target/release/plugin-api /app/eldoria-hub/plugin-api

# Copy static files (will be populated by frontend build)
RUN mkdir -p /app/static

EXPOSE 8080

CMD ["./eldoria-hub"]