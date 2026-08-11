use anyhow::Context;

struct Config {
    discord_bot_token: String,
    discord_channel_id: u64,
    poll_question: String,
    poll_options: Vec<String>,
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
            poll_duration_hours,
            poll_allow_multiselect,
            uptime_kuma_push_url,
        })
    }
}

fn main() {
    dotenvy::dotenv().ok();

    match Config::from_env() {
        Ok(config) => {
            println!("parsed config ok:");
            println!("  discord_channel_id = {}", config.discord_channel_id);
            println!("  poll_question = {:?}", config.poll_question);
            println!("  poll_options = {:?}", config.poll_options);
            println!("  poll_duration_hours = {}", config.poll_duration_hours);
            println!("  poll_allow_multiselect = {}", config.poll_allow_multiselect);
            println!("  uptime_kuma_push_url = {:?}", config.uptime_kuma_push_url);
        }
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    }
}
