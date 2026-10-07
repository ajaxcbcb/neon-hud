//! Bounded, read-only import of a user-selected iCalendar file.
use std::{collections::{HashMap, HashSet}, fs::File, io::Read, path::Path};

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rrule::{RRule, RRuleSet, Tz, Unvalidated};

use super::{Availability, Calendar, CalendarEvent};

const MAX_BYTES: u64 = 1_048_576;
const MAX_EVENTS: usize = 512;
const MAX_OCCURRENCES: usize = 256;
const DAY_MS: i64 = 86_400_000;

#[derive(Default)]
struct Event {
    uid: String,
    title: String,
    start: Option<DateTime<Tz>>,
    end: Option<DateTime<Tz>>,
    all_day: bool,
    rule: Option<String>,
    rdates: Vec<DateTime<Tz>>,
    exdates: Vec<DateTime<Tz>>,
    recurrence_id: Option<DateTime<Tz>>,
    canceled: bool,
}

pub(super) fn read(path: &Path, start_ms: i64, end_ms: i64) -> Calendar {
    let source = path.display().to_string();
    match load(path, start_ms, end_ms) {
        Ok(events) => Calendar { status: Availability::Ready, source, events },
        Err(message) => Calendar { status: Availability::Error(format!("Calendar import: {message}")), source, events: Vec::new() },
    }
}

fn load(path: &Path, start_ms: i64, end_ms: i64) -> Result<Vec<CalendarEvent>, String> {
    if end_ms <= start_ms || end_ms - start_ms > 7 * DAY_MS {
        return Err("date window must be at most seven days".into());
    }
    if !path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("ics")) {
        return Err("select a local .ics file".into());
    }
    let file = File::open(path).map_err(|e| format!("cannot open file: {e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("path is not a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|e| format!("cannot read file: {e}"))?;
    if bytes.len() as u64 > MAX_BYTES { return Err("file exceeds 1 MiB".into()); }
    let content = String::from_utf8(bytes).map_err(|_| "file is not UTF-8")?;
    let entries = parse(&content)?;
    let mut canceled_uids = HashSet::new();
    let mut master_uids = HashSet::new();
    let mut exceptions: HashMap<(String, i64), &Event> = HashMap::new();
    for event in &entries {
        if event.canceled && event.recurrence_id.is_none() { canceled_uids.insert(event.uid.as_str()); }
        if let Some(id) = event.recurrence_id {
            if exceptions.insert((event.uid.clone(), id.timestamp_millis()), event).is_some() {
                return Err("duplicate recurrence override".into());
            }
        } else if !event.canceled && !master_uids.insert(event.uid.as_str()) {
            return Err("duplicate VEVENT UID".into());
        }
    }
    let mut output = Vec::new();
    let mut occurrence_count = 0usize;
    let mut used_overrides = HashSet::new();
    for event in &entries {
        if event.recurrence_id.is_some() || event.canceled || canceled_uids.contains(event.uid.as_str()) { continue; }
        let start = event.start.ok_or("VEVENT has no DTSTART")?;
        let end = event.end.ok_or("VEVENT has no DTEND")?;
        let duration = end.signed_duration_since(start);
        if duration < Duration::zero() { return Err("VEVENT ends before it starts".into()); }
        let occurrences = if let Some(rule_text) = &event.rule {
            let rule: RRule<Unvalidated> = rule_text.parse().map_err(|e| format!("invalid RRULE: {e}"))?;
            let rule = rule.validate(start).map_err(|e| format!("invalid RRULE: {e}"))?;
            let mut set = RRuleSet::new(start).rrule(rule);
            for &date in &event.rdates { set = set.rdate(date); }
            for &date in &event.exdates { set = set.exdate(date); }
            let after = Utc.timestamp_millis_opt(start_ms.saturating_sub(duration.num_milliseconds()).saturating_sub(1))
                .single().ok_or("invalid date window")?.with_timezone(&Tz::UTC);
            let before = Utc.timestamp_millis_opt(end_ms).single().ok_or("invalid date window")?.with_timezone(&Tz::UTC);
            let result = set.after(after).before(before).all((MAX_OCCURRENCES + 1) as u16);
            if result.limited || result.dates.len() > MAX_OCCURRENCES { return Err("recurrence exceeds 256 occurrences in window".into()); }
            result.dates
        } else if !event.rdates.is_empty() || !event.exdates.is_empty() {
            let mut set = RRuleSet::new(start).rdate(start);
            for &date in &event.rdates { set = set.rdate(date); }
            for &date in &event.exdates { set = set.exdate(date); }
            let after = Utc.timestamp_millis_opt(start_ms.saturating_sub(duration.num_milliseconds()).saturating_sub(1))
                .single().ok_or("invalid date window")?.with_timezone(&Tz::UTC);
            let before = Utc.timestamp_millis_opt(end_ms).single().ok_or("invalid date window")?.with_timezone(&Tz::UTC);
            let result = set.after(after).before(before).all((MAX_OCCURRENCES + 1) as u16);
            if result.limited || result.dates.len() > MAX_OCCURRENCES { return Err("recurrence exceeds 256 occurrences in window".into()); }
            result.dates
        } else { vec![start] };
        for occurrence in occurrences {
            occurrence_count += 1;
            if occurrence_count > MAX_OCCURRENCES { return Err("window exceeds 256 occurrences".into()); }
            let key = (event.uid.clone(), occurrence.timestamp_millis());
            if let Some(override_event) = exceptions.get(&key) {
                used_overrides.insert(key);
                if !override_event.canceled { push_event(&mut output, override_event, override_event.start.ok_or("override has no DTSTART")?, override_event.end.ok_or("override has no DTEND")?, start_ms, end_ms)?; }
            } else {
                let finish = if event.all_day {
                    let days = end.naive_local().date().signed_duration_since(start.naive_local().date()).num_days();
                    let date = occurrence.naive_local().date().checked_add_signed(Duration::days(days)).ok_or("date overflow")?;
                    occurrence.timezone().from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or("date overflow")?)
                        .single().ok_or("ambiguous or nonexistent all-day boundary")?
                } else { occurrence.checked_add_signed(duration).ok_or("date overflow")? };
                push_event(&mut output, event, occurrence, finish, start_ms, end_ms)?;
            }
        }
    }
    // An override may move an occurrence into the window from outside it.
    for (key, event) in exceptions {
        if used_overrides.contains(&key) || event.canceled || canceled_uids.contains(event.uid.as_str()) { continue; }
        push_event(&mut output, event, event.start.ok_or("override has no DTSTART")?, event.end.ok_or("override has no DTEND")?, start_ms, end_ms)?;
    }
    output.sort_by_key(|event| event.starts_at_ms);
    Ok(output)
}

fn push_event(output: &mut Vec<CalendarEvent>, event: &Event, start: DateTime<Tz>, end: DateTime<Tz>, window_start: i64, window_end: i64) -> Result<(), String> {
    let starts_at_ms = start.timestamp_millis();
    let ends_at_ms = end.timestamp_millis();
    if ends_at_ms <= window_start || starts_at_ms >= window_end { return Ok(()); }
    if output.len() >= MAX_EVENTS { return Err("window exceeds 512 events".into()); }
    output.push(CalendarEvent { title: event.title.clone(), starts_at_ms, ends_at_ms, all_day: event.all_day });
    Ok(())
}

fn parse(content: &str) -> Result<Vec<Event>, String> {
    let mut lines: Vec<String> = Vec::new();
    for physical in content.trim_start_matches('\u{feff}').lines() {
        let line = physical.trim_end_matches('\r');
        if line.starts_with(' ') || line.starts_with('\t') {
            let previous = lines.last_mut().ok_or("folded line has no predecessor")?;
            previous.push_str(&line[1..]);
        } else { lines.push(line.to_owned()); }
    }
    if lines.first().map(String::as_str) != Some("BEGIN:VCALENDAR") || lines.last().map(String::as_str) != Some("END:VCALENDAR") {
        return Err("not a complete VCALENDAR".into());
    }
    let mut events = Vec::new();
    let mut current: Option<Event> = None;
    let mut nested = 0usize;
    for line in lines.iter().skip(1).take(lines.len() - 2) {
        if line == "BEGIN:VEVENT" {
            if current.is_some() { return Err("nested VEVENT".into()); }
            current = Some(Event::default());
            continue;
        }
        if line == "END:VEVENT" {
            if nested != 0 { return Err("unterminated VEVENT component".into()); }
            let event = current.take().ok_or("END:VEVENT without BEGIN:VEVENT")?;
            if event.uid.is_empty() { return Err("VEVENT has no UID".into()); }
            if events.len() >= MAX_EVENTS { return Err("file exceeds 512 VEVENTs".into()); }
            events.push(event);
            continue;
        }
        let Some(event) = current.as_mut() else { continue; };
        if line.starts_with("BEGIN:") { nested += 1; continue; }
        if line.starts_with("END:") { nested = nested.checked_sub(1).ok_or("invalid component nesting")?; continue; }
        if nested != 0 { continue; }
        let (property, value) = line.split_once(':').ok_or("malformed VEVENT property")?;
        let name = property.split(';').next().unwrap_or("").to_ascii_uppercase();
        match name.as_str() {
            "UID" => event.uid = value.to_owned(),
            "SUMMARY" => event.title = unescape(value)?,
            "DTSTART" => { let (date, all_day) = parse_date(property, value)?; event.start = Some(date); event.all_day = all_day; }
            "DTEND" => event.end = Some(parse_date(property, value)?.0),
            "RRULE" => { if event.rule.replace(value.to_owned()).is_some() { return Err("multiple RRULEs unsupported".into()); } }
            "RDATE" | "EXDATE" => {
                if property.to_ascii_uppercase().contains("VALUE=PERIOD") { return Err("RDATE periods unsupported".into()); }
                let target = if name == "RDATE" { &mut event.rdates } else { &mut event.exdates };
                for part in value.split(',') {
                    if target.len() >= MAX_OCCURRENCES { return Err("too many RDATE/EXDATE values".into()); }
                    target.push(parse_date(property, part)?.0);
                }
            }
            "RECURRENCE-ID" => {
                if property.to_ascii_uppercase().contains("RANGE=") { return Err("recurrence RANGE overrides unsupported".into()); }
                event.recurrence_id = Some(parse_date(property, value)?.0);
            }
            "STATUS" => event.canceled = value.eq_ignore_ascii_case("CANCELLED"),
            "EXRULE" | "DURATION" => return Err(format!("{name} unsupported")),
            _ => {}
        }
    }
    if current.is_some() { return Err("unterminated VEVENT".into()); }
    for event in &mut events {
        if event.canceled && event.start.is_none() { continue; }
        let start = event.start.ok_or("VEVENT has no DTSTART")?;
        if event.end.is_none() {
            event.end = Some(if event.all_day {
                let next = start.naive_local().date().succ_opt().ok_or("date overflow")?;
                start.timezone().from_local_datetime(&next.and_hms_opt(0, 0, 0).ok_or("date overflow")?)
                    .single().ok_or("ambiguous or nonexistent all-day boundary")?
            } else { start });
        }
    }
    Ok(events)
}

fn parse_date(property: &str, value: &str) -> Result<(DateTime<Tz>, bool), String> {
    let mut timezone = None;
    let mut date_value = false;
    for parameter in property.split(';').skip(1) {
        let (key, val) = parameter.split_once('=').ok_or("malformed date parameter")?;
        if key.eq_ignore_ascii_case("TZID") {
            let parsed: chrono_tz::Tz = val.trim_matches('"').parse().map_err(|_| format!("unsupported TZID {val}"))?;
            timezone = Some(Tz::Tz(parsed));
        } else if key.eq_ignore_ascii_case("VALUE") {
            if !val.eq_ignore_ascii_case("DATE") && !val.eq_ignore_ascii_case("DATE-TIME") { return Err("unsupported date value type".into()); }
            date_value = val.eq_ignore_ascii_case("DATE");
        }
    }
    if value.len() == 8 || date_value {
        if value.len() != 8 { return Err("invalid all-day date".into()); }
        let date = NaiveDate::parse_from_str(value, "%Y%m%d").map_err(|_| "invalid all-day date")?;
        let local = date.and_hms_opt(0, 0, 0).ok_or("invalid all-day date")?;
        let zone = timezone.unwrap_or(Tz::Local(Local));
        return Ok((zone.from_local_datetime(&local).single().ok_or("ambiguous or nonexistent local date")?, true));
    }
    let utc = value.ends_with('Z');
    let raw = if utc { &value[..value.len() - 1] } else { value };
    let local = NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S").map_err(|_| "invalid date-time")?;
    let zone = if utc { Tz::UTC } else { timezone.unwrap_or(Tz::Local(Local)) };
    if utc && timezone.is_some() { return Err("UTC date-time cannot also have TZID".into()); }
    Ok((zone.from_local_datetime(&local).single().ok_or("ambiguous or nonexistent local time")?, false))
}

fn unescape(value: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' { out.push(ch); continue; }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some('\\') => out.push('\\'),
            Some(';') => out.push(';'),
            Some(',') => out.push(','),
            _ => return Err("invalid SUMMARY escape".into()),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    fn events(body: &str, start: &str, end: &str) -> Result<Vec<CalendarEvent>, String> {
        let text = format!("BEGIN:VCALENDAR\nVERSION:2.0\n{body}\nEND:VCALENDAR");
        let entries = parse(&text)?;
        // Tests use the same expansion path as read without touching private files.
        let path = std::env::temp_dir().join(format!("neon-calendar-{}-{}.ics", std::process::id(), NEXT_FILE.fetch_add(1, Ordering::Relaxed)));
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        let result = load(&path, start.parse().unwrap(), end.parse().unwrap());
        let _ = std::fs::remove_file(path);
        assert!(!entries.is_empty());
        result
    }

    #[test]
    fn utc_folding_and_escaped_summary() {
        let e = events("BEGIN:VEVENT\nUID:one\nDTSTART:20261007T010000Z\nDTEND:20261007T020000Z\nSUMMARY:Line\\nTwo\\,\n continued\nEND:VEVENT", "1791331200000", "1791417600000").unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].title, "Line\nTwo,continued");
    }

    #[test]
    fn all_day_and_timezone() {
        let e = events("BEGIN:VEVENT\nUID:day\nDTSTART;VALUE=DATE:20261007\nSUMMARY:Day\nEND:VEVENT\nBEGIN:VEVENT\nUID:zone\nDTSTART;TZID=Asia/Singapore:20261007T090000\nDTEND;TZID=Asia/Singapore:20261007T100000\nSUMMARY:Zone\nEND:VEVENT", "1791331200000", "1791417600000").unwrap();
        assert_eq!(e.len(), 2);
        assert!(e.iter().any(|x| x.all_day));
        assert!(e.iter().any(|x| x.title == "Zone" && x.starts_at_ms == 1791334800000));
    }

    #[test]
    fn all_day_without_dtend_uses_next_local_midnight_across_dst_fall() {
        let start = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).single().unwrap().timestamp_millis();
        let end = Utc.with_ymd_and_hms(2026, 11, 4, 0, 0, 0).single().unwrap().timestamp_millis();
        let e = events("BEGIN:VEVENT\nUID:dst-day\nDTSTART;TZID=America/New_York;VALUE=DATE:20261101\nRRULE:FREQ=DAILY;COUNT=2\nSUMMARY:Day\nEND:VEVENT", &start.to_string(), &end.to_string()).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].ends_at_ms - e[0].starts_at_ms, 25 * 3_600_000);
        assert_eq!(e[1].ends_at_ms - e[1].starts_at_ms, 24 * 3_600_000);
    }

    #[test]
    fn recurrence_exdate_and_override() {
        let e = events("BEGIN:VEVENT\nUID:series\nDTSTART:20261007T090000Z\nDTEND:20261007T100000Z\nRRULE:FREQ=DAILY;COUNT=3\nEXDATE:20261008T090000Z\nSUMMARY:Daily\nEND:VEVENT\nBEGIN:VEVENT\nUID:series\nRECURRENCE-ID:20261009T090000Z\nDTSTART:20261009T110000Z\nDTEND:20261009T120000Z\nSUMMARY:Moved\nEND:VEVENT", "1791331200000", "1791763200000").unwrap();
        assert_eq!(e.len(), 2);
        assert!(e.iter().any(|x| x.title == "Moved"));
    }

    #[test]
    fn malformed_and_bounds() {
        assert!(parse("BEGIN:VCALENDAR\nEND:VCALENDAR").is_ok());
        assert!(parse("BEGIN:VCALENDAR\nBEGIN:VEVENT\nEND:VCALENDAR").is_err());
        assert!(unescape("bad\\x").is_err());
        assert!(load(Path::new("not.ics"), 0, 8 * DAY_MS).is_err());
    }

    #[test]
    fn canceled_occurrence_without_dtstart() {
        let e = events("BEGIN:VEVENT\nUID:series\nDTSTART:20261007T090000Z\nDTEND:20261007T100000Z\nRRULE:FREQ=DAILY;COUNT=2\nSUMMARY:Daily\nEND:VEVENT\nBEGIN:VEVENT\nUID:series\nRECURRENCE-ID:20261008T090000Z\nSTATUS:CANCELLED\nEND:VEVENT", "1791331200000", "1791504000000").unwrap();
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn rejects_growing_or_oversized_file() {
        let path = std::env::temp_dir().join(format!("neon-calendar-{}-{}.ics", std::process::id(), NEXT_FILE.fetch_add(1, Ordering::Relaxed)));
        std::fs::write(&path, vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        let result = load(&path, 0, DAY_MS);
        let _ = std::fs::remove_file(path);
        assert!(result.unwrap_err().contains("1 MiB"));
    }
}
