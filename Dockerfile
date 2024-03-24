FROM rust:1.77 as builder
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM debian:bookworm-slim
# because the rust diesel need using the lib libpq.so.5 from libpq5.
RUN apt-get update && apt-get install libpq5 -y && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ntex-restful-api-server /
CMD ["./ntex-restful-api-server"]