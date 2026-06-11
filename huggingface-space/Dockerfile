FROM rust:1.78-slim-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --no-default-features --features "web-server,petgraph_backend" --bin webserver

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/webserver /usr/local/bin/hipcortex-webserver
EXPOSE 3030
ENV RUST_LOG=info
CMD ["hipcortex-webserver"]
