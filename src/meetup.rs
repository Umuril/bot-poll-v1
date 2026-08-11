use anyhow::{anyhow, Context};
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;

struct Property {
    name: String,
    params: String,
    value: String,
}

fn parse_property(line: &str) -> Option<Property> {
    let colon_idx = line.find(':')?;
    let head = &line[..colon_idx];
    let value = line[colon_idx + 1..].to_string();
    let (name, params) = match head.find(';') {
        Some(semi_idx) => (head[..semi_idx].to_string(), head[semi_idx..].to_string()),
        None => (head.to_string(), String::new()),
    };
    Some(Property { name, params, value })
}

fn extract_tzid(params: &str) -> Option<String> {
    for part in params.split(';') {
        if let Some(tzid) = part.strip_prefix("TZID=") {
            if !tzid.is_empty() {
                return Some(tzid.to_string());
            }
        }
    }
    None
}

fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw_line in text.split("\r\n").flat_map(|l| l.split('\n')) {
        if (raw_line.starts_with(' ') || raw_line.starts_with('\t')) && !lines.is_empty() {
            let last = lines.last_mut().expect("checked non-empty above");
            last.push_str(&raw_line[1..]);
        } else if !raw_line.is_empty() {
            lines.push(raw_line.to_string());
        }
    }
    lines
}

fn vevent_blocks(lines: &[String]) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in lines {
        if line == "BEGIN:VEVENT" {
            current = Some(Vec::new());
        } else if line == "END:VEVENT" {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
        } else if let Some(block) = current.as_mut() {
            block.push(line.clone());
        }
    }
    blocks
}

fn event_is_today(dtstart: &Property, now: DateTime<Utc>) -> anyhow::Result<bool> {
    let raw_value = dtstart.value.trim_end_matches('Z');
    let naive = NaiveDateTime::parse_from_str(raw_value, "%Y%m%dT%H%M%S")
        .with_context(|| format!("unrecognized DTSTART value {:?}", dtstart.value))?;

    let (event_date, today) = match extract_tzid(&dtstart.params) {
        Some(tzid_name) => {
            let tz: Tz = tzid_name
                .parse()
                .map_err(|_| anyhow!("unrecognized DTSTART TZID {tzid_name:?}"))?;
            let localized = naive
                .and_local_timezone(tz)
                .single()
                .with_context(|| format!("ambiguous or invalid local time {naive} in {tzid_name}"))?;
            (localized.date_naive(), now.with_timezone(&tz).date_naive())
        }
        None => (naive.date(), now.date_naive()),
    };

    Ok(event_date == today)
}

fn parse_event(block: &[String], now: DateTime<Utc>) -> anyhow::Result<Option<String>> {
    let mut dtstart_prop: Option<Property> = None;
    let mut url_value: Option<String> = None;

    for line in block {
        if let Some(prop) = parse_property(line) {
            match prop.name.as_str() {
                "DTSTART" => dtstart_prop = Some(prop),
                "URL" => url_value = Some(prop.value.clone()),
                _ => {}
            }
        }
    }

    let dtstart_prop = dtstart_prop.context("VEVENT block is missing a DTSTART property")?;
    let url = url_value.context("VEVENT block is missing a URL property")?;

    if event_is_today(&dtstart_prop, now)? {
        Ok(Some(url))
    } else {
        Ok(None)
    }
}

fn matching_links(ical_text: &str, now: DateTime<Utc>) -> anyhow::Result<Vec<String>> {
    let lines = unfold(ical_text);
    let blocks = vevent_blocks(&lines);
    let mut links = Vec::new();
    for block in &blocks {
        if let Some(url) = parse_event(block, now)? {
            links.push(url);
        }
    }
    Ok(links)
}

pub async fn todays_event_links(ical_url: &str, now: DateTime<Utc>) -> anyhow::Result<Vec<String>> {
    let client = reqwest::Client::new();
    let response = client
        .get(ical_url)
        .send()
        .await
        .with_context(|| format!("failed to fetch iCal feed from {ical_url}"))?
        .error_for_status()
        .with_context(|| format!("iCal feed at {ical_url} returned an error status"))?;

    let body = response
        .text()
        .await
        .context("failed to read iCal feed response body")?;

    matching_links(&body, now)
}
