# Use official Rust image as builder
FROM rust:1.75 as builder

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Create src directory and copy source
COPY src/ src/
COPY migrations/ migrations/
COPY proto/ proto/
COPY schemas/ schemas/

# Build the application
RUN cargo build --release --features "postgres_backend,temporal_indexing,web_server"

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app hipcortex

# Set working directory
WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/hipcortex /usr/local/bin/hipcortex

# Copy configuration files
COPY --chown=hipcortex:hipcortex migrations/ migrations/
COPY --chown=hipcortex:hipcortex schemas/ schemas/

# Create data directory
RUN mkdir -p /app/data && chown hipcortex:hipcortex /app/data

# Switch to app user
USER hipcortex

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set default environment variables
ENV RUST_LOG=info
ENV API_PORT=8080
ENV DATA_DIR=/app/data

# Run the application
CMD ["hipcortex"]
