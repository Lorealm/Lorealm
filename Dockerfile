FROM rust:1-bookworm AS server
WORKDIR /src
COPY backend .
RUN cargo build --release -p lorealm_server

FROM oven/bun:1 AS web
WORKDIR /src/frontend
COPY frontend .
COPY sdk/js ../sdk/js
RUN bun install --frozen-lockfile \
 && cd packages/ui && bun run prepack \
 && cd ../../apps/web && bun run build

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
 && rm -rf /var/lib/apt/lists/*
COPY --from=server /src/target/release/lorealm_server /usr/local/bin/lorealm
COPY --from=web /src/frontend/apps/web/dist /usr/share/lorealm/web

# Seeded into the volume on first start; everything else falls back to defaults.
RUN printf 'hosting:\n  ip: 0.0.0.0\n  serve_frontend: true\n  frontend_dir: /usr/share/lorealm/web\n' \
  > /usr/share/lorealm/config.yaml

WORKDIR /lorealm
VOLUME /lorealm
EXPOSE 1551
CMD ["sh", "-c", "cp -n /usr/share/lorealm/config.yaml . ; exec lorealm"]
