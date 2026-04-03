# Build stage
FROM rust:1.88.0-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy common source (shared protobuf definitions)
COPY src/common/ ./src/common/

# Copy rocksdbservice source
COPY src/server/rocksdbservice/ ./src/server/rocksdbservice/

# Build the RocksDB service
WORKDIR /app/src/server/rocksdbservice
RUN cargo build --release

# Runtime stage
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false app

# Create data directory
RUN mkdir -p /data && chown app:app /data

# Copy binary
COPY --from=builder /app/src/server/rocksdbservice/target/release/rocksdbservice /usr/local/bin/rocksdbservice

# Set permissions
RUN chmod +x /usr/local/bin/rocksdbservice

# Switch to app user
USER app

# Expose gRPC port
EXPOSE 47007

# Set default data path
ENV ROCKSDB_PATH=/data

# Start the gRPC service
CMD ["rocksdbservice", "--path", "/data", "--addr", "0.0.0.0", "--port", "47007"]