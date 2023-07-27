FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin moco

FROM debian:bookworm-slim

RUN adduser --disabled-password app
USER app

WORKDIR /app

COPY --from=builder /app/target/release/moco /usr/local/bin
ENTRYPOINT ["/usr/local/bin/moco"]
