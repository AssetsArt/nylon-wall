use chrono::Datelike;
use nylon_wall_common::zone::Schedule;

/// Check if a schedule is currently active based on the current time.
pub fn is_schedule_active(schedule: &Schedule) -> bool {
    let now = chrono::Local::now();

    // chrono: Mon=0..Sun=6 matches our Schedule.days convention
    let weekday = now.weekday().num_days_from_monday() as u8;
    if !schedule.days.contains(&weekday) {
        return false;
    }

    let current_time = now.format("%H:%M").to_string();
    let start = &schedule.start_time;
    let end = &schedule.end_time;

    if start <= end {
        // Normal range: e.g. 08:00 - 18:00
        current_time >= *start && current_time < *end
    } else {
        // Overnight range: e.g. 22:00 - 06:00
        current_time >= *start || current_time < *end
    }
}

/// Check if a policy should be evaluated right now based on its optional schedule.
/// Returns true if there's no schedule (always active) or the schedule is currently active.
pub fn is_policy_active(schedule: &Option<Schedule>) -> bool {
    match schedule {
        None => true,
        Some(sched) => is_schedule_active(sched),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_schedule_always_active() {
        assert!(is_policy_active(&None));
    }

    #[test]
    fn test_schedule_all_days_wide_range() {
        let schedule = Schedule {
            days: vec![0, 1, 2, 3, 4, 5, 6],
            start_time: "00:00".to_string(),
            end_time: "23:59".to_string(),
        };
        assert!(is_schedule_active(&schedule));
    }

    #[test]
    fn test_schedule_no_days() {
        let schedule = Schedule {
            days: vec![],
            start_time: "00:00".to_string(),
            end_time: "23:59".to_string(),
        };
        assert!(!is_schedule_active(&schedule));
    }

    #[test]
    fn test_overnight_schedule() {
        let schedule = Schedule {
            days: vec![0, 1, 2, 3, 4, 5, 6],
            start_time: "22:00".to_string(),
            end_time: "06:00".to_string(),
        };
        // This depends on current time, so we just check it doesn't panic
        let _ = is_schedule_active(&schedule);
    }
}
