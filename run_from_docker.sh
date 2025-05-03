export RUST_LOG=debug && \
docker compose down && \
docker compose up --build
# docker build -t canvas . --no-cache && \
# docker run -p 3001:3001 canvas