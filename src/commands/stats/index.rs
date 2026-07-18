use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::entry::repository::EntryMetadata;

pub const MAX_WEEKS: usize = 60;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Year(pub u32);

pub type YearWeekdayIndex = HashMap<(Year, chrono::Weekday), [u32; MAX_WEEKS]>;

fn iso_week_monday(date: NaiveDate) -> NaiveDate {
    let iso = date.iso_week();
    NaiveDate::from_isoywd_opt(iso.year(), iso.week(), Weekday::Mon)
        .expect("ISO week Monday always exists")
}

fn first_week_monday(year: i32) -> NaiveDate {
    iso_week_monday(NaiveDate::from_ymd_opt(year, 1, 1).expect("Jan 1 always exists"))
}

pub fn n_calendar_weeks(year: i32) -> usize {
    let first = first_week_monday(year);
    let last =
        iso_week_monday(NaiveDate::from_ymd_opt(year, 12, 31).expect("Dec 31 always exists"));
    (last - first).num_days() as usize / 7 + 1
}

fn calendar_week_index(date: NaiveDate) -> (i32, usize) {
    let cy = date.year();
    let first = first_week_monday(cy);
    let week_monday = iso_week_monday(date);
    let idx = (week_monday - first).num_days() as usize / 7;
    (cy, idx)
}

pub fn cell_date(year: i32, week_idx: usize, weekday: Weekday) -> NaiveDate {
    let first = first_week_monday(year);
    let offset = i64::from(weekday.number_from_monday() - 1);
    first + Duration::days(week_idx as i64 * 7 + offset)
}

pub fn cell_belongs_to_year(year: i32, week_idx: usize, weekday: Weekday) -> bool {
    cell_date(year, week_idx, weekday).year() == year
}

pub fn month_week_ranges(year: i32, n_weeks: u32) -> [(u32, u32); 12] {
    let mut ranges = [(u32::MAX, 0u32); 12];
    let first = first_week_monday(year);
    for week_idx in 0..n_weeks {
        let monday = first + Duration::days(i64::from(week_idx) * 7);
        let thursday = monday + Duration::days(3);
        if thursday.year() != year {
            continue;
        }
        let month = thursday.month0() as usize;
        let w = week_idx + 1;
        if w < ranges[month].0 {
            ranges[month].0 = w;
        }
        if w > ranges[month].1 {
            ranges[month].1 = w;
        }
    }
    ranges
}

pub fn build_year_weekday_index(metadata: &[EntryMetadata]) -> YearWeekdayIndex {
    let mut index: YearWeekdayIndex = HashMap::new();
    for entry in metadata {
        let (year, week) = calendar_week_index(entry.timestamp.date_naive());
        let weekday = entry.timestamp.weekday();
        let slot = index
            .entry((Year(year as u32), weekday))
            .or_insert([0; MAX_WEEKS]);
        slot[week] += u32::try_from(entry.size.div_ceil(1024)).unwrap_or(u32::MAX);
    }
    index
}

pub fn last_used_week(index: &YearWeekdayIndex, year: i32) -> Option<usize> {
    let n_weeks = n_calendar_weeks(year);
    let weekdays = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    for idx in (0..n_weeks).rev() {
        for weekday in &weekdays {
            if let Some(line) = index.get(&(Year(year as u32), *weekday))
                && line[idx] > 0
            {
                return Some(idx + 1);
            }
        }
    }
    None
}

pub fn year_span(metadata: &[EntryMetadata]) -> Option<(i32, i32)> {
    let earliest = metadata.iter().map(|x| x.timestamp.year()).min()?;
    let latest = metadata.iter().map(|x| x.timestamp.year()).max()?;
    Some((earliest, latest))
}

pub const WEEKDAY_LIST: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];
