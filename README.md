# bot-poll-v1

A one-shot Rust bot that posts a fixed poll to a Discord channel, meant to
be run on a cron schedule via [Dokploy](https://dokploy.com)'s Schedule
feature. It does one thing per run: read config from environment
variables, post one poll message, exit.

The container's own main process is deliberately idle (`sleep infinity`) —
Dokploy's Schedule ("Application Job") feature runs commands via `docker
exec` into an already-running container on a cron tick, it does not start
a fresh one-shot container per run. Running the bot binary itself as the
container's main process causes a restart loop (exits → Dokploy restarts
the Application → posts again → exits → ...), which reposts the poll
repeatedly instead of once per schedule. See "Deploying on Dokploy" below
for the correct two-resource setup.

## Prerequisites

- Rust (`cargo`) for local development
- Docker, for building the deployable image
- A Discord bot application with a token (create one at
  https://discord.com/developers/applications, add a Bot, copy its Token)
- The bot must be invited to your server with the "Send Messages" permission
  on the target channel
- If using `MEETUP_ICAL_URL` (see below), the bot also needs the "Mention
  @everyone, @here, and All Roles" permission on the target channel, or the
  follow-up message will post but not actually notify anyone
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

The image's own `CMD` just keeps the container alive (`sleep infinity`) —
running the bot means exec-ing the binary into that running container,
the same way Dokploy's Schedule feature will:

```bash
docker build -t bot-poll-v1:local .
docker run -d --name bot-poll-v1 --env-file .env bot-poll-v1:local
docker exec bot-poll-v1 /usr/local/bin/bot-poll-v1
```

## Deploying on Dokploy

Dokploy's Schedule feature runs its command via `docker exec` into an
**already-running** container — it does not spin up a fresh one-shot
container per cron tick. So deployment is two resources working together:
an Application that stays idle, and a Schedule that execs the bot inside it.

1. Create a new **Application** in Dokploy, pointing at this repo (or a
   registry image built from this repo's `Dockerfile`). Deploy it — its
   container will start and just sit idle (`sleep infinity`); that's expected.
2. In the Application's environment variables panel, set all variables from
   `.env.example`:
   - `DISCORD_BOT_TOKEN`, `DISCORD_CHANNEL_ID`, `POLL_QUESTION`,
     `POLL_OPTIONS` (required)
   - `POLL_DURATION_HOURS`, `POLL_ALLOW_MULTISELECT`, `POLL_OPTION_EMOJIS`
     (optional, sensible defaults apply if unset)
   - `UPTIME_KUMA_PUSH_URL` (optional, see Monitoring below)
3. Create a new **Schedule** in Dokploy, type **Application Job**, attached
   to that Application.
4. Set the cron expression for how often the poll should post (e.g. daily
   at 9am), and set the command to `/usr/local/bin/bot-poll-v1`.
5. Save. Dokploy will `docker exec` the binary into the running Application
   container on your cron schedule; each run's exit code and logs show up
   in the Schedule's run history. The exec inherits the Application's
   configured env vars automatically — no separate env config needed on
   the Schedule itself.

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

This push monitor only tells you about the outcome of the last scheduled
run. To separately watch whether the idle Application container itself is
up right now (independent of the cron schedule), add a second, unrelated
Kuma monitor of type **Docker Container** pointed at that container — no
push URL or bot code involved, Kuma polls the Docker daemon directly. This
needs Kuma to have a Docker Host configured (local `/var/run/docker.sock`
if Kuma runs on the same server, or the remote TCP/HTTP Docker API
otherwise).

## Same-day meetup link (optional)

If `MEETUP_ICAL_URL` is set to a Meetup group's iCal feed URL (e.g.
`https://www.meetup.com/<group-slug>/events/ical/`), the bot checks that
feed after posting the poll. For any event whose start date is today (in
the event's own timezone, as given in the feed), it posts a follow-up
channel message: `@everyone @here <event link>`.

- If unset, this step is skipped entirely — no behavior change.
- If set, a fetch or parse failure on the feed fails the whole run (same
  as a Discord API error) — the run exits non-zero and, if configured,
  pings Uptime Kuma with `status=down`.
- The bot needs the **"Mention @everyone, @here, and All Roles"**
  permission on the target channel for the mentions in the follow-up
  message to actually notify members, rather than post as inert text.
- Running the bot more than once on the same day posts the follow-up
  message that many times — there's no dedup/state, consistent with the
  bot's stateless, one-shot-per-invocation design.
