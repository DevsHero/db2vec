###############################################################################
# 1. Planner stage: generate the dependency recipe (with C++ compiler)
###############################################################################
FROM rustlang/rust:nightly-bullseye-slim AS chef

# Install system deps including C++ compiler
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      build-essential \                       
      pkg-config \
      libssl-dev \
      git \
      ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Install cargo-chef and sccache
RUN cargo install --locked cargo-chef sccache
ENV RUSTC_WRAPPER="sccache" \
    SCCACHE_DIR="/sccache"

WORKDIR /app

# Copy manifests and dummy main for cargo-chef
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo chef prepare --recipe-path recipe.json

###############################################################################
# 2. Builder stage: compile dependencies & your code
###############################################################################
FROM chef AS builder
WORKDIR /app

# Rehydrate dependencies
COPY --from=chef /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

# Build the full application
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release

###############################################################################
# 3. Runtime stage: minimal Debian image
###############################################################################
FROM debian:bullseye-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/db2vec /usr/local/bin/db2vec

# Drop privileges: non-root user
RUN useradd --system --uid 10001 --shell /usr/sbin/nologin appuser
USER appuser

ENTRYPOINT ["/usr/local/bin/db2vec"]
