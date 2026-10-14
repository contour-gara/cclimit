use crate::api::Limit;
use jiff::Timestamp;

pub fn render(limits: &[Limit], now: Timestamp) -> String {
    limits
        .iter()
        .map(|limit| match limit.resets_at {
            Some(resets_at) => {
                format!(
                    "{} {}% {}",
                    limit.label,
                    limit.percent,
                    between(now, resets_at)
                )
            }
            None => format!("{} {}%", limit.label, limit.percent),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn between(from: Timestamp, to: Timestamp) -> String {
    format_duration(to.as_second() - from.as_second())
}

fn format_duration(secs: i64) -> String {
    let (days, hours, mins) = (secs / 86400, secs % 86400 / 3600, secs % 3600 / 60);
    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{mins}m")
    } else if mins > 0 {
        format!("{mins}m")
    } else {
        "<1m".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::limit;
    use pretty_assertions::assert_eq;

    const NOW: &str = "2026-10-13T19:15:00Z";

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn duration_of_a_day_or_more_shows_days_and_hours() {
        assert_eq!(format_duration(3 * 86400 + 17 * 3600 + 59 * 60), "3d17h");
    }

    #[test]
    fn duration_of_an_hour_or_more_shows_hours_and_minutes() {
        assert_eq!(format_duration(2 * 3600 + 15 * 60 + 59), "2h15m");
    }

    #[test]
    fn duration_of_a_minute_or_more_shows_minutes() {
        assert_eq!(format_duration(45 * 60), "45m");
    }

    #[test]
    fn duration_under_a_minute_shows_less_than_one_minute() {
        assert_eq!(format_duration(59), "<1m");
    }

    #[test]
    fn renders_each_limit_with_time_left() {
        let limits = [
            limit("5h", 11, Some("2026-10-13T21:30:00Z")),
            limit("7d", 38, Some("2026-10-17T12:15:00Z")),
        ];
        assert_eq!(render(&limits, ts(NOW)), "5h 11% 2h15m | 7d 38% 3d17h");
    }

    #[test]
    fn missing_reset_renders_without_time_left() {
        assert_eq!(render(&[limit("7d", 38, None)], ts(NOW)), "7d 38%");
    }
}
