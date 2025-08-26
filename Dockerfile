# Multi-stage build for tops-worker
FROM rust:1.88-slim as builder

# Install system dependencies for OpenCL and OpenSSL
RUN apt-get update && apt-get install -y \
	ocl-icd-opencl-dev \
	opencl-headers \
	pkg-config \
	libssl-dev \
	&& rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files for dependency caching
COPY Cargo.toml ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --features gpu

# Remove dummy main.rs and copy actual source code
RUN rm src/main.rs
COPY src/ ./src/

# Build the actual application
RUN cargo build --release --features gpu

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies for OpenCL
RUN apt-get update && apt-get install -y \
	ocl-icd-libopencl1 \
	ocl-icd-opencl-dev \
	opencl-headers \
	ca-certificates \
	curl \
	&& rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -r worker && useradd -r -g worker worker

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/tops-worker /app/tops-worker

# Copy any additional runtime files
COPY README.md ./

# Create necessary directories
RUN mkdir -p /app/logs && chown -R worker:worker /app

# Switch to non-root user
USER worker

# Expose health and metrics port
EXPOSE 8082

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
	CMD curl -f http://localhost:8082/health || exit 1

# Default command
CMD ["/app/tops-worker"]
