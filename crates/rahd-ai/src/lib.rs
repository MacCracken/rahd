//! Rahd AI — natural language event parsing and smart scheduling
//!
//! Parses inputs like "lunch with Sam tomorrow at noon" into structured event data.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("empty input")]
    EmptyInput,
    #[error("could not parse event from: {0}")]
    Unparseable(String),
}

/// Result of parsing a natural language event description.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedEvent {
    pub title: String,
    pub date: Option<String>,
    pub time: Option<String>,
    pub duration_mins: Option<u32>,
    pub attendees: Vec<String>,
    pub location: Option<String>,
}

impl ParsedEvent {
    /// Convert parsed date/time hints into actual DateTimes.
    /// Uses `reference` as "now" for relative dates like "tomorrow".
    pub fn to_datetimes(&self, reference: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let date = self.resolve_date(reference);
        let time = self.resolve_time();
        let start = date.and_time(time).and_utc();
        let duration = Duration::minutes(self.duration_mins.unwrap_or(60) as i64);
        let end = start + duration;
        (start, end)
    }

    fn resolve_date(&self, reference: DateTime<Utc>) -> NaiveDate {
        match self.date.as_deref() {
            Some("today") => reference.date_naive(),
            Some("tomorrow") => reference.date_naive() + Duration::days(1),
            Some(s) => {
                // Try parsing as YYYY-MM-DD
                if let Ok(d) = s.parse::<NaiveDate>() {
                    return d;
                }
                // Try "March 20" style
                reference.date_naive()
            }
            None => reference.date_naive(),
        }
    }

    fn resolve_time(&self) -> NaiveTime {
        match self.time.as_deref() {
            Some("noon") => NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            Some("midnight") => NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            Some(s) => {
                // "3pm", "3:30pm", "15:00"
                let s = s.to_lowercase();
                let is_pm = s.contains("pm");
                let is_am = s.contains("am");
                let cleaned = s.replace("pm", "").replace("am", "");
                if let Some((h, m)) = cleaned.split_once(':') {
                    let mut hour: u32 = h.trim().parse().unwrap_or(9);
                    let min: u32 = m.trim().parse().unwrap_or(0);
                    if is_pm && hour < 12 {
                        hour += 12;
                    }
                    if is_am && hour == 12 {
                        hour = 0;
                    }
                    NaiveTime::from_hms_opt(hour, min, 0)
                        .unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
                } else {
                    let mut hour: u32 = cleaned.trim().parse().unwrap_or(9);
                    if is_pm && hour < 12 {
                        hour += 12;
                    }
                    if is_am && hour == 12 {
                        hour = 0;
                    }
                    NaiveTime::from_hms_opt(hour, 0, 0)
                        .unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
                }
            }
            None => NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        }
    }
}

/// Natural language event parser.
pub struct NlEventParser;

impl NlEventParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a natural language event description.
    pub fn parse_event(&self, input: &str) -> Result<ParsedEvent, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        let lower = input.to_lowercase();
        let tokens: Vec<&str> = lower.split_whitespace().collect();

        let mut date: Option<String> = None;
        let mut time: Option<String> = None;
        let mut duration_mins: Option<u32> = None;
        let mut attendees: Vec<String> = Vec::new();
        let mut location: Option<String> = None;

        // Track which token indices are consumed by metadata.
        let mut consumed = vec![false; tokens.len()];

        // Parse "tomorrow", "today"
        for (i, tok) in tokens.iter().enumerate() {
            if *tok == "tomorrow" {
                date = Some("tomorrow".to_string());
                consumed[i] = true;
            } else if *tok == "today" {
                date = Some("today".to_string());
                consumed[i] = true;
            }
        }

        // Parse "on <day>" (e.g. "on friday")
        for i in 0..tokens.len().saturating_sub(1) {
            if tokens[i] == "on" && is_day_name(tokens[i + 1]) {
                date = Some(tokens[i + 1].to_string());
                consumed[i] = true;
                consumed[i + 1] = true;
            }
        }

        // Parse time patterns: "at noon", "at 3pm", "at 15:00"
        for i in 0..tokens.len().saturating_sub(1) {
            if tokens[i] == "at" && is_time_like(tokens[i + 1]) {
                time = Some(tokens[i + 1].to_string());
                consumed[i] = true;
                consumed[i + 1] = true;
            }
        }

        // Parse standalone time words (noon, midnight) if not already found
        if time.is_none() {
            for (i, tok) in tokens.iter().enumerate() {
                if (*tok == "noon" || *tok == "midnight") && !consumed[i] {
                    time = Some(tok.to_string());
                    consumed[i] = true;
                }
            }
        }

        // Parse "for X hours/minutes"
        for i in 0..tokens.len().saturating_sub(2) {
            if tokens[i] == "for"
                && let Ok(n) = tokens[i + 1].parse::<u32>()
            {
                let unit = tokens[i + 2];
                if unit.starts_with("hour") {
                    duration_mins = Some(n * 60);
                    consumed[i] = true;
                    consumed[i + 1] = true;
                    consumed[i + 2] = true;
                } else if unit.starts_with("min") {
                    duration_mins = Some(n);
                    consumed[i] = true;
                    consumed[i + 1] = true;
                    consumed[i + 2] = true;
                }
            }
        }

        // Parse "with <name>"
        for i in 0..tokens.len().saturating_sub(1) {
            if tokens[i] == "with" && !consumed[i] {
                // Collect names until a keyword
                consumed[i] = true;
                let mut j = i + 1;
                while j < tokens.len() && !is_keyword(tokens[j]) && !consumed[j] {
                    // Capitalize the name from original input
                    let orig_tokens: Vec<&str> = input.split_whitespace().collect();
                    if j < orig_tokens.len() {
                        attendees.push(capitalize(orig_tokens[j]));
                    }
                    consumed[j] = true;
                    j += 1;
                }
            }
        }

        // Parse location: "at <place>" where place is not a time
        // Look for "at" tokens that weren't consumed (i.e., not time)
        for i in 0..tokens.len().saturating_sub(1) {
            if tokens[i] == "at" && !consumed[i] {
                let mut loc_parts = Vec::new();
                consumed[i] = true;
                let orig_tokens: Vec<&str> = input.split_whitespace().collect();
                let mut j = i + 1;
                while j < tokens.len() && !is_keyword(tokens[j]) && !consumed[j] {
                    if j < orig_tokens.len() {
                        loc_parts.push(orig_tokens[j]);
                    }
                    consumed[j] = true;
                    j += 1;
                }
                if !loc_parts.is_empty() {
                    location = Some(loc_parts.join(" "));
                }
            }
        }

        // Build title from unconsumed tokens
        let orig_tokens: Vec<&str> = input.split_whitespace().collect();
        let title_parts: Vec<&str> = orig_tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| !consumed[*i])
            .map(|(_, t)| *t)
            .collect();

        let title = if title_parts.is_empty() {
            input.to_string()
        } else {
            title_parts.join(" ")
        };

        Ok(ParsedEvent {
            title,
            date,
            time,
            duration_mins,
            attendees,
            location,
        })
    }
}

impl Default for NlEventParser {
    fn default() -> Self {
        Self::new()
    }
}

fn is_time_like(s: &str) -> bool {
    if s == "noon" || s == "midnight" {
        return true;
    }
    let cleaned = s.replace("pm", "").replace("am", "");
    // "3", "3:30", "15:00"
    if cleaned.contains(':') {
        let parts: Vec<&str> = cleaned.split(':').collect();
        return parts.len() == 2
            && parts[0].trim().parse::<u32>().is_ok()
            && parts[1].trim().parse::<u32>().is_ok();
    }
    cleaned.trim().parse::<u32>().is_ok()
        && (s.contains("pm")
            || s.contains("am")
            || cleaned.trim().parse::<u32>().unwrap_or(99) <= 23)
}

fn is_day_name(s: &str) -> bool {
    matches!(
        s,
        "monday"
            | "tuesday"
            | "wednesday"
            | "thursday"
            | "friday"
            | "saturday"
            | "sunday"
            | "mon"
            | "tue"
            | "wed"
            | "thu"
            | "fri"
            | "sat"
            | "sun"
    )
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "at" | "on" | "for" | "with" | "from" | "to" | "in" | "tomorrow" | "today"
    )
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

/// Scores events by urgency/priority.
pub struct PriorityScorer;

impl PriorityScorer {
    /// Score an event (higher = more urgent).
    ///
    /// - Sooner events score higher
    /// - Events with attendees score higher (social commitment)
    /// - Recurring events score slightly lower (can be rescheduled)
    pub fn score(event: &rahd_core::Event, now: DateTime<Utc>) -> f64 {
        let hours_until = (event.start - now).num_hours().max(0) as f64;
        // Base score: inversely proportional to time until event
        let time_score = 100.0 / (1.0 + hours_until);

        // Attendees bonus: +10 per attendee
        let attendee_bonus = event.attendees.len() as f64 * 10.0;

        // Recurrence penalty: -5 for recurring events
        let recurrence_penalty = if event.recurrence.is_some() { 5.0 } else { 0.0 };

        time_score + attendee_bonus - recurrence_penalty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rahd_core::Event;
    use uuid::Uuid;

    #[test]
    fn parse_lunch_tomorrow_noon() {
        let parser = NlEventParser::new();
        let result = parser.parse_event("lunch tomorrow at noon").unwrap();
        assert_eq!(result.title, "lunch");
        assert_eq!(result.date.as_deref(), Some("tomorrow"));
        assert_eq!(result.time.as_deref(), Some("noon"));
    }

    #[test]
    fn parse_meeting_with_attendee() {
        let parser = NlEventParser::new();
        let result = parser
            .parse_event("meeting with Bob on Friday at 3pm for 1 hour")
            .unwrap();
        assert_eq!(result.title, "meeting");
        assert!(result.attendees.contains(&"Bob".to_string()));
        assert_eq!(result.time.as_deref(), Some("3pm"));
        assert_eq!(result.duration_mins, Some(60));
    }

    #[test]
    fn parse_time_formats_pm() {
        let parser = NlEventParser::new();
        let result = parser.parse_event("call at 3pm").unwrap();
        assert_eq!(result.time.as_deref(), Some("3pm"));

        let parsed = result;
        let reference = Utc.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap();
        let (start, _) = parsed.to_datetimes(reference);
        assert_eq!(start.hour(), 15);
    }

    #[test]
    fn parse_time_24h() {
        let parser = NlEventParser::new();
        let result = parser.parse_event("standup at 15:00").unwrap();
        let reference = Utc.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap();
        let (start, _) = result.to_datetimes(reference);
        assert_eq!(start.hour(), 15);
    }

    #[test]
    fn parse_duration_minutes() {
        let parser = NlEventParser::new();
        let result = parser.parse_event("break for 30 minutes").unwrap();
        assert_eq!(result.duration_mins, Some(30));
    }

    #[test]
    fn parse_duration_hours() {
        let parser = NlEventParser::new();
        let result = parser.parse_event("workshop for 2 hours").unwrap();
        assert_eq!(result.duration_mins, Some(120));
    }

    #[test]
    fn parse_attendees() {
        let parser = NlEventParser::new();
        let result = parser.parse_event("lunch with Alice").unwrap();
        assert_eq!(result.attendees, vec!["Alice".to_string()]);
    }

    #[test]
    fn parse_location() {
        let parser = NlEventParser::new();
        let result = parser
            .parse_event("dinner at 7pm at The Restaurant")
            .unwrap();
        assert_eq!(result.time.as_deref(), Some("7pm"));
        assert_eq!(result.location.as_deref(), Some("The Restaurant"));
    }

    #[test]
    fn parse_empty_input_error() {
        let parser = NlEventParser::new();
        assert!(parser.parse_event("").is_err());
    }

    #[test]
    fn parse_title_only() {
        let parser = NlEventParser::new();
        let result = parser.parse_event("dentist").unwrap();
        assert_eq!(result.title, "dentist");
        assert!(result.date.is_none());
        assert!(result.time.is_none());
    }

    #[test]
    fn priority_scorer_sooner_is_higher() {
        let now = Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap();
        let soon = Event {
            id: Uuid::new_v4(),
            title: "Soon".to_string(),
            description: None,
            start: now + Duration::hours(1),
            end: now + Duration::hours(2),
            location: None,
            attendees: vec![],
            recurrence: None,
            reminders: vec![],
            calendar_id: "default".to_string(),
            created_at: now,
            updated_at: now,
        };
        let later = Event {
            start: now + Duration::hours(24),
            end: now + Duration::hours(25),
            title: "Later".to_string(),
            ..soon.clone()
        };
        assert!(PriorityScorer::score(&soon, now) > PriorityScorer::score(&later, now));
    }

    #[test]
    fn priority_scorer_attendees_boost() {
        let now = Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap();
        let solo = Event {
            id: Uuid::new_v4(),
            title: "Solo".to_string(),
            description: None,
            start: now + Duration::hours(2),
            end: now + Duration::hours(3),
            location: None,
            attendees: vec![],
            recurrence: None,
            reminders: vec![],
            calendar_id: "default".to_string(),
            created_at: now,
            updated_at: now,
        };
        let group = Event {
            attendees: vec!["Alice".to_string(), "Bob".to_string()],
            title: "Group".to_string(),
            ..solo.clone()
        };
        assert!(PriorityScorer::score(&group, now) > PriorityScorer::score(&solo, now));
    }

    use chrono::Timelike;
}
