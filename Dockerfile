FROM rust:1-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

LABEL org.opencontainers.image.title="rpg2java-transpiler"
LABEL org.opencontainers.image.description="Offline RPG to Java commercial migration PoC CLI"
LABEL org.opencontainers.image.source="https://github.com/kurumonn/rpg2java-transpiler"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates openjdk-21-jdk-headless \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rpg2java-transpiler /usr/local/bin/rpg2java

WORKDIR /work
ENTRYPOINT ["rpg2java"]
CMD ["doctor"]
