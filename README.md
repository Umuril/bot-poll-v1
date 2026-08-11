# bot-poll-v1

A one-shot Rust bot that posts a fixed poll to a Discord channel, meant to
be run on a cron schedule via [Dokploy](https://dokploy.com)'s Schedule
feature. It does one thing per run: read config from environment
variables, post one poll message, exit.

## Prerequisites

- Rust (`cargo`) for local development
- Docker, for building the deployable image
- A Discord bot application with a token (create one at
  https://discord.com/developers/applications, add a Bot, copy its Token)
- The bot must be invited to your server with the "Send Messages" permission
  on the target channel
- (Optional) A self-hosted [Uptime Kuma](https://github.com/louislam/uptime-kuma)
  instance with a "Push" monitor, for heartbeat monitoring

## Local development

```bash
cp .env.example .env
# fill in DISCORD_BOT_TOKEN, DISCORD_CHANNEL_ID, POLL_QUESTION, POLL_OPTIONS
cargo run
```

Check the target Discord channel for the poll.

## Docker

```bash
docker build -t bot-poll-v1:local .
docker run --rm --env-file .env bot-poll-v1:local
```

## Deploying on Dokploy

1. Create a new **Schedule** resource in Dokploy, pointing at this repo
   (or a registry image built from this repo's `Dockerfile`).
2. Set the cron expression for how often the poll should post (e.g. daily
   at 9am).
3. In the Schedule's environment variables panel, set all variables from
   `.env.example`:
   - `DISCORD_BOT_TOKEN`, `DISCORD_CHANNEL_ID`, `POLL_QUESTION`,
     `POLL_OPTIONS` (required)
   - `POLL_DURATION_HOURS`, `POLL_ALLOW_MULTISELECT`, `POLL_OPTION_EMOJIS`
     (optional, sensible defaults apply if unset)
   - `UPTIME_KUMA_PUSH_URL` (optional, see Monitoring below)
4. Save. Dokploy will run the container on your cron schedule; each run's
   exit code and logs show up in the Schedule's run history.

## Monitoring (optional)

Dokploy's run history only tells you the outcome *if the container ran at
all*. To also catch the schedule silently not firing (misconfigured cron,
crashed container, host down), point the bot at a self-hosted Uptime Kuma
**Push monitor**:

1. In Kuma, create a new monitor of type **Push**, with a "heartbeat
   interval" a bit longer than your poll's cron interval (e.g. cron is
   daily → set the interval to ~26 hours so a late run doesn't false-alarm).
2. Copy the monitor's push URL into `UPTIME_KUMA_PUSH_URL`.
3. On Kuma's monitor, add a **Discord webhook notification**: create a
   webhook on an alerts channel in Discord (Channel Settings → Integrations
   → Webhooks), paste its URL into Kuma's notification settings.
4. The bot pings the push URL with `status=up` on success and `status=down`
   on failure; if no ping arrives at all within the heartbeat interval,
   Kuma marks the monitor down and posts to your Discord alerts channel.
