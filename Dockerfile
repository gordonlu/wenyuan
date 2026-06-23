FROM rust:1.86-slim-bookworm AS backend
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build -p wenyuan-server --release && \
    cp target/release/wenyuan-server /wenyuan-server && \
    rm -rf target

FROM node:22-slim AS frontend
WORKDIR /app
COPY web/package.json web/pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY web .
RUN pnpm build

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=backend /wenyuan-server /usr/local/bin/wenyuan-server
COPY --from=frontend /app/dist /app/web/dist
WORKDIR /app
EXPOSE 3210
ENV WENYUAN_BIND=0.0.0.0:3210
CMD ["wenyuan-server"]
