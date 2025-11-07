FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim

WORKDIR /app

COPY --from=builder /app/target/release/freshrss-share /app/freshrss-share
COPY templates /app/templates
COPY static /app/static
COPY Rocket.toml /app/Rocket.toml

RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app

USER appuser

EXPOSE 8000

CMD ["/app/freshrss-share"]
