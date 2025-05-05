FROM rust:1.86-slim-bookworm as builder

WORKDIR /app

# openssl stuff
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

RUN cargo build --release

# Stage 2: Runtime
FROM gcr.io/distroless/cc-debian12

WORKDIR /workserver

COPY --from=builder /app/target/release/server ./server

COPY ./public ./public
COPY ./js ./js

EXPOSE 3001
CMD ["./server"]