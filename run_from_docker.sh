cargo build --release  && \
docker build -t canvas . --no-cache && \
docker run -p 3001:3001 canvas