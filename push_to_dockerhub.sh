cargo build --release && \
docker build -t canvas . && \
docker tag canvas lgxerxes/canvas && \
docker push lgxerxes/canvas