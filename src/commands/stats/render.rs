use std::fmt::Write;

use chrono::{Datelike, NaiveDate, Weekday};
use dialoguer::console;

use super::heatmap::Heatmap;
use super::index::{MAX_WEEKS, WEEKDAY_LIST, Year, YearWeekdayIndex, month_week_ranges};

const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn year_block(
    index: &YearWeekdayIndex,
    heatmap: &Heatmap,
    year: i32,
    today: NaiveDate,
) -> String {
    if super::index::last_used_week(index, year).is_none() {
        return String::new();
    }
    let n_weeks = super::index::n_calendar_weeks(year);
    let mut block = String::new();
    let _ = writeln!(block, "{}", year_header_line(year, n_weeks));
    let _ = writeln!(block, "{}", month_header_line(year, n_weeks));
    for weekday in &WEEKDAY_LIST {
        let data = weekday_line(index, heatmap, Year(year as u32), *weekday, n_weeks, today);
        let _ = writeln!(block, "{weekday} {data}");
    }
    block
}

pub(super) fn year_header_line(year: i32, n_weeks: usize) -> String {
    let year_str = year.to_string();
    let total_width = n_weeks * 3;
    let padding = total_width.saturating_sub(year_str.len());
    let left = padding / 2;
    let right = padding - left;
    format!("{}{}{}", " ".repeat(left), year_str, " ".repeat(right))
}

pub(super) fn month_header_line(year: i32, n_weeks: usize) -> String {
    let ranges = month_week_ranges(year, n_weeks as u32);
    let total_width = n_weeks * 3;
    let mut line = String::new();
    for (i, abbr) in MONTH_ABBR.iter().enumerate() {
        let (first_week, last_week) = ranges[i];
        if first_week > last_week {
            continue;
        }
        let midpoint = u32::midpoint(first_week, last_week);
        let pos = (midpoint as usize).saturating_sub(1) * 3;
        let start = if pos + abbr.len() > total_width {
            total_width.saturating_sub(abbr.len())
        } else {
            pos
        };
        while line.len() < start {
            line.push(' ');
        }
        line.push_str(abbr);
    }
    while line.len() < total_width {
        line.push(' ');
    }
    line
}

fn weekday_line(
    index: &YearWeekdayIndex,
    heatmap: &Heatmap,
    year: Year,
    weekday: Weekday,
    n_weeks: usize,
    today: NaiveDate,
) -> String {
    let line = index.get(&(year, weekday)).unwrap_or(&[0u32; MAX_WEEKS]);
    let mut output = String::new();
    for week_num in 1..=n_weeks {
        let week_idx = week_num - 1;
        if !super::index::cell_belongs_to_year(year.0 as i32, week_idx, weekday) {
            output.push_str("   ");
            continue;
        }
        let count = line[week_idx];
        if let Some((bg, fg, text)) = heatmap.cell(count) {
            let _ = write!(output, "{}", console::style(text).bg(bg).fg(fg));
        } else {
            let cell_date = super::index::cell_date(year.0 as i32, week_idx, weekday);
            if cell_date > today && cell_date.year() == today.year() {
                output.push_str("   ");
            } else {
                let bg = Heatmap::empty_cell_bg();
                let _ = write!(output, "{}", console::style("   ").bg(bg));
            }
        }
    }
    output
}
