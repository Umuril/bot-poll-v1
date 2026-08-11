use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use icalendar::{Calendar, CalendarDateTime, Component, DatePerhapsTime};

fn event_is_today(start: &DatePerhapsTime, now: DateTime<Utc>) -> anyhow::Result<bool> {
    let event_date = start.date_naive();

    let today = match start {
        DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { tzid, .. }) => {
            let tz: Tz = tzid
                .parse()
                .map_err(|_| anyhow!("unrecognized DTSTART TZID {tzid:?}"))?;
            now.with_timezone(&tz).date_naive()
        }
        DatePerhapsTime::DateTime(CalendarDateTime::Utc(_))
        | DatePerhapsTime::DateTime(CalendarDateTime::Floating(_))
        | DatePerhapsTime::Date(_) => now.date_naive(),
    };

    Ok(event_date == today)
}

fn matching_links(ical_text: &str, now: DateTime<Utc>) -> anyhow::Result<Vec<String>> {
    let calendar: Calendar = ical_text
        .parse()
        .map_err(|err: String| anyhow!("failed to parse iCal feed: {err}"))?;

    let mut links = Vec::new();
    for event in calendar.events() {
        let start = event
            .get_start()
            .context("VEVENT is missing a usable DTSTART property")?;
        let url = event
            .property_value("URL")
            .context("VEVENT is missing a URL property")?
            .to_string();

        if event_is_today(&start, now)? {
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
