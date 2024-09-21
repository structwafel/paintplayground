#!/bin/bash

# Check if the git repository is dirty
if [[ -n $(git status --porcelain) ]]; then
  echo "Git repository is dirty. Please commit or stash your changes."
  exit 1
fi

# Run cargo check
if ! cargo check; then
  echo "Cargo check failed. Please fix the issues."
  exit 1
fi

# Run cargo test to make sure the tests pass
if ! cargo test; then
  echo "Cargo test failed. Please fix the issues."
  exit 1
fi

# Run cargo build
if ! cargo build --release; then
  echo "Cargo build failed. Please fix the issues."
  exit 1
fi

# build image and push to docker hub
docker build -t canvas . --no-cache && \
docker tag canvas lgxerxes/canvas && \
docker push lgxerxes/canvas

# Build and push the Docker image
# docker build -t registry.gitlab.com/structwafel/paintsandbox . --no-cache && \
# docker push registry.gitlab.com/structwafel/paintsandbox