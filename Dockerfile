# Build stage
FROM rust:1.76-buster as builder

WORKDIR /app

# accept the build argument
ARG DATABASE_URL

ENV DATABASE_URL=$DATABASE_URL

COPY . .

RUN cargo build --release

# Run stage
FROM debian:bookworm-slim

WORKDIR /usr/local/bin

COPY --from=builder /app/target/release/master-backend .

CMD ["./master-backend"]
