FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Dummy web/dist for include_dir! (client embeds the file explorer frontend)
RUN mkdir -p web/dist && echo '<html></html>' > web/dist/index.html
RUN cargo build --release --bin tunelo

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /src/target/release/tunelo /usr/local/bin/tunelo
EXPOSE 8080 4433/udp
ENTRYPOINT ["tunelo"]
CMD ["relay", "--domain", "localhost", "--tunnel-addr", "0.0.0.0:4433", "--http-addr", "0.0.0.0:8080"]
