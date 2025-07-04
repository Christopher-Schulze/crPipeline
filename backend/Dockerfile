FROM rust:1.76 as build
WORKDIR /app
COPY backend ./
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd -m app
WORKDIR /app
COPY --from=build /app/target/release/backend /usr/local/bin/backend
COPY --from=build /app/target/release/worker /usr/local/bin/worker
COPY --from=build /app/target/release/cleanup /usr/local/bin/cleanup
COPY backend/migrations ./migrations
RUN chown -R app:app /app
USER app
ENV RUST_LOG=info
CMD ["backend"]
