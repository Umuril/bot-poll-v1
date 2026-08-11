FROM rust:1-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bot-poll-v1 /usr/local/bin/bot-poll-v1

# The container's main process is deliberately idle. Dokploy's Schedule
# ("Application Job") feature runs commands via `docker exec` into an
# already-running container on a cron tick — it does not start a fresh
# one-shot container per run. So this container just stays alive, and the
# Schedule job execs /usr/local/bin/bot-poll-v1 inside it on the cron
# expression. See README.md's "Deploying on Dokploy" section.
CMD ["sleep", "infinity"]
