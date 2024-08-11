cargo build --release && \
docker build -t canvas . --no-cache && \
docker tag canvas lgxerxes/canvas && \
docker push lgxerxes/canvas