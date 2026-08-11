use std::time::Duration;

use anyhow::Context;
use serenity::builder::{CreateMessage, CreatePoll, CreatePollAnswer};
use serenity::http::Http;
use serenity::model::id::{ChannelId, MessageId};

struct Config {
    discord_bot_token: String,
    discord_channel_id: u64,
    poll_question: String,
    poll_options: Vec<String>,
    poll_option_emojis: Vec<String>,
    poll_duration_hours: u16,
    poll_allow_multiselect: bool,
    uptime_kuma_push_url: Option<String>,
}

fn require_env(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("{key} is not set"))
}

impl Config {
    fn from_env() -> anyhow::Result<Self> {
        let discord_bot_token = require_env("DISCORD_BOT_TOKEN")?;

        let discord_channel_id_raw = require_env("DISCORD_CHANNEL_ID")?;
        let discord_channel_id: u64 = discord_channel_id_raw.parse().with_context(|| {
            format!(
                "DISCORD_CHANNEL_ID must be a valid numeric channel ID, got {discord_channel_id_raw:?}"
            )
        })?;
        if discord_channel_id == 0 {
            anyhow::bail!("DISCORD_CHANNEL_ID must not be 0");
        }

        let poll_question = require_env("POLL_QUESTION")?;

        let poll_options_raw = require_env("POLL_OPTIONS")?;
        let poll_options: Vec<String> = poll_options_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if poll_options.len() < 2 || poll_options.len() > 10 {
            anyhow::bail!(
                "POLL_OPTIONS must contain between 2 and 10 comma-separated options, got {}",
                poll_options.len()
            );
        }

        let poll_option_emojis: Vec<String> = match std::env::var("POLL_OPTION_EMOJIS") {
            Ok(raw) => {
                let emojis: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
                if emojis.len() != poll_options.len() {
                    anyhow::bail!(
                        "POLL_OPTION_EMOJIS must have exactly one entry per POLL_OPTIONS option ({} expected, got {})",
                        poll_options.len(),
                        emojis.len()
                    );
                }
                emojis
            }
            Err(_) => vec![String::new(); poll_options.len()],
        };

        let poll_duration_hours: u16 = match std::env::var("POLL_DURATION_HOURS") {
            Ok(raw) => raw.parse().with_context(|| {
                format!("POLL_DURATION_HOURS must be a valid number, got {raw:?}")
            })?,
            Err(_) => 24,
        };
        if !(1..=768).contains(&poll_duration_hours) {
            anyhow::bail!(
                "POLL_DURATION_HOURS must be between 1 and 768, got {poll_duration_hours}"
            );
        }

        let poll_allow_multiselect = match std::env::var("POLL_ALLOW_MULTISELECT") {
            Ok(raw) => raw.parse().with_context(|| {
                format!("POLL_ALLOW_MULTISELECT must be true or false, got {raw:?}")
            })?,
            Err(_) => false,
        };

        let uptime_kuma_push_url = std::env::var("UPTIME_KUMA_PUSH_URL").ok();

        Ok(Config {
            discord_bot_token,
            discord_channel_id,
            poll_question,
            poll_options,
            poll_option_emojis,
            poll_duration_hours,
            poll_allow_multiselect,
            uptime_kuma_push_url,
        })
    }
}

async fn send_poll(config: &Config) -> anyhow::Result<MessageId> {
    let http = Http::new(&config.discord_bot_token);

    let answers: Vec<CreatePollAnswer> = config
        .poll_options
        .iter()
        .zip(config.poll_option_emojis.iter())
        .map(|(text, emoji)| {
            let answer = CreatePollAnswer::new().text(text.as_str());
            if emoji.is_empty() {
                answer
            } else {
                answer.emoji(emoji.clone())
            }
        })
        .collect();

    let poll = CreatePoll::new()
        .question(config.poll_question.as_str())
        .answers(answers)
        .duration(Duration::from_secs(
            u64::from(config.poll_duration_hours) * 3600,
        ));

    let poll = if config.poll_allow_multiselect {
        poll.allow_multiselect()
    } else {
        poll
    };

    let channel_id = ChannelId::new(config.discord_channel_id);
    let message = channel_id
        .send_message(&http, CreateMessage::new().poll(poll))
        .await
        .context("failed to send poll message to Discord")?;

    Ok(message.id)
}

async fn ping_kuma(push_url: &str, up: bool, msg: &str) {
    let status = if up { "up" } else { "down" };
    let client = reqwest::Client::new();
    let result = client
        .get(push_url)
        .query(&[("status", status), ("msg", msg)])
        .send()
        .await;

    if let Err(err) = result {
        eprintln!("warning: failed to ping Uptime Kuma heartbeat: {err}");
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    };

    match send_poll(&config).await {
        Ok(message_id) => {
            println!("poll posted successfully, message id: {message_id}");
            if let Some(push_url) = &config.uptime_kuma_push_url {
                ping_kuma(push_url, true, "OK").await;
            }
        }
        Err(err) => {
            eprintln!("error: {err:#}");
            if let Some(push_url) = &config.uptime_kuma_push_url {
                ping_kuma(push_url, false, &format!("{err:#}")).await;
            }
            std::process::exit(1);
        }
    }
}
