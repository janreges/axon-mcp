# Multi-stage Docker build for MCP Server
FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace and dependency files
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/
COPY database/ ./database/
COPY mcp-protocol/ ./mcp-protocol/
COPY mcp-server/ ./mcp-server/
COPY mocks/ ./mocks/

# Build the release binary
RUN cargo build --release --bin mcp-server

# Runtime stage with minimal dependencies
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -r -s /bin/false mcp-server

# Copy the binary from builder stage
COPY --from=builder /app/target/release/mcp-server /usr/local/bin/mcp-server

# Set appropriate permissions
RUN chmod +x /usr/local/bin/mcp-server

# Create data directory for database
RUN mkdir -p /data && chown mcp-server:mcp-server /data

# Switch to non-root user
USER mcp-server

# Set working directory
WORKDIR /data

# Expose the default port
EXPOSE 3000

# Set default environment variables
ENV DATABASE_URL=sqlite:///data/db.sqlite
ENV LISTEN_ADDR=0.0.0.0
ENV LOG_LEVEL=info

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the server
CMD ["mcp-server"]