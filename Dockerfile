# Use official Rust image as builder
FROM rust:1.87-bookworm AS builder

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml ./
COPY build.rs ./

# Copy Cargo.lock if it exists (create empty one if not)
RUN if [ ! -f Cargo.lock ]; then touch Cargo.lock; fi

# Create src directory and copy source
COPY src/ src/
COPY migrations/ migrations/
COPY proto/ proto/
COPY schemas/ schemas/

# Create empty benchmark files to satisfy Cargo.toml
RUN mkdir -p benches && \
    echo 'fn main() {}' > benches/temporal_indexer_bench.rs && \
    echo 'fn main() {}' > benches/symbolic_store_bench.rs

# Build with web-server feature (petgraph_backend is default, no external deps)
RUN cargo build --release --bin webserver --no-default-features --features "web-server,petgraph_backend"

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies (curl for HEALTHCHECK; libssl3 for TLS)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app hipcortex

# Set working directory
WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/webserver /usr/local/bin/webserver

# Copy configuration files
COPY --chown=hipcortex:hipcortex migrations/ migrations/
COPY --chown=hipcortex:hipcortex schemas/ schemas/

# Create data directory
RUN mkdir -p /app/data && chown hipcortex:hipcortex /app/data

# Switch to app user
USER hipcortex

# Expose port
EXPOSE 3030

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3030/health || exit 1

# Set default environment variables
ENV RUST_LOG=info
ENV API_PORT=3030
ENV DATA_DIR=/app/data

# Run the application
CMD ["webserver"]
