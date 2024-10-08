# set environment to debug
export RUST_LOG=debug && \
cargo build --release  && \
docker build -t canvas . --no-cache && \
docker run -p 3001:3001 canvas