FROM gcr.io/distroless/cc

# Set the working directory
WORKDIR /workserver

# Copy the pre-built binary
COPY ./target/release/server .
# COPY ./target/x86_64-unknown-linux-musl/release/server .

# Copy any other required files
COPY ./public ./public

# Set the startup command to run your binary
EXPOSE 3001
CMD ["./server"]