FROM rust:alpine3.21 AS build

RUN apk update && \
    apk upgrade --no-cache && \
    apk add --no-cache lld mold musl musl-dev libc-dev cmake clang clang-dev openssl file \
        libressl-dev git make build-base bash curl wget zip gnupg coreutils gcc g++  zstd binutils upx



WORKDIR /paintplayground
COPY . ./
RUN cargo build --release


FROM scratch

COPY --from=build /paintplayground/target/release/server /bin/server
COPY --from=build /paintplayground/public /paintplayground/public

WORKDIR /paintplayground
ENTRYPOINT ["/bin/server"]