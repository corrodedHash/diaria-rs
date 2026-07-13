#![allow(
    clippy::cast_possible_wrap,
    clippy::as_conversions,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

mod heatmap;
mod index;
mod render;

use dialoguer::console;

use crate::entry::repository::DiariaEntryRepository;
use crate::util::stdout_printer::UserOutput;

pub struct Command {
    repository: Box<dyn DiariaEntryRepository>,
    user_output: Box<dyn UserOutput>,
}

impl Command {
    pub fn new(
        repository: Box<dyn DiariaEntryRepository>,
        user_output: Box<dyn UserOutput>,
    ) -> Self {
        Self {
            repository,
            user_output,
        }
    }

    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = self.repository.list_entry_metadata();
        if metadata.is_empty() {
            return Ok(());
        }
        let Some((earliest_year, latest_year)) = index::year_span(&metadata) else {
            return Ok(());
        };

        let heatmap = heatmap::Heatmap::new();
        let built_index = index::build_year_weekday_index(&metadata);
        let mut result = String::new();
        for year in earliest_year..=latest_year {
            result += &render::year_block(&built_index, &heatmap, year);
        }

        console::set_colors_enabled(true);
        console::set_true_colors_enabled(true);
        self.user_output.print(&result);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::entry::repository::{EntryMetadata, MockDiariaEntryRepository};
    use crate::util::stdout_printer::MockUserOutput;

    use chrono::{Datelike as _, NaiveDate, NaiveDateTime, TimeZone, Weekday};
    use dialoguer::console;

    fn local_from(date: &str) -> chrono::DateTime<chrono::Local> {
        let naive = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S").unwrap();
        chrono::Local.from_local_datetime(&naive).single().unwrap()
    }

    use super::Command;
    use super::heatmap::Heatmap;
    use super::index::{
        MAX_WEEKS, Year, YearWeekdayIndex, build_year_weekday_index, last_used_week,
        month_week_ranges,
    };
    use super::render::{month_header_line, year_block};

    fn generate_multi_year_metadata() -> Vec<EntryMetadata> {
        let mut entries = Vec::new();
        for year in 2023..=2026 {
            let max_month = if year == 2026 { 6 } else { 12 };
            for month in 1..=max_month {
                for (day, size) in [(1, 2048), (10, 5120), (20, 3072)] {
                    let naive = NaiveDateTime::parse_from_str(
                        &format!("{year:04}-{month:02}-{day:02}T12:00:00"),
                        "%Y-%m-%dT%H:%M:%S",
                    )
                    .unwrap();
                    if let Some(ts) = chrono::Local.from_local_datetime(&naive).single() {
                        entries.push(EntryMetadata {
                            timestamp: ts,
                            size,
                        });
                    }
                }
                for time in ["12:00:00", "18:00:00"] {
                    let naive = NaiveDateTime::parse_from_str(
                        &format!("{year:04}-{month:02}-15T{time}"),
                        "%Y-%m-%dT%H:%M:%S",
                    )
                    .unwrap();
                    if let Some(ts) = chrono::Local.from_local_datetime(&naive).single() {
                        entries.push(EntryMetadata {
                            timestamp: ts,
                            size: if time == "12:00:00" { 5000 } else { 7000 },
                        });
                    }
                }
            }
        }
        entries
    }

    #[test]
    fn test_stats_execute_with_mocked_entries() {
        let mut repo = MockDiariaEntryRepository::new();
        let metadata = vec![
            EntryMetadata {
                timestamp: local_from("2026-06-21T16:50:46"),
                size: 1024,
            },
            EntryMetadata {
                timestamp: local_from("2026-06-22T16:50:46"),
                size: 2048,
            },
        ];
        repo.expect_list_entry_metadata()
            .return_once(move || metadata);

        let mut user_output = MockUserOutput::new();
        user_output
            .expect_print()
            .withf(|s| !s.is_empty())
            .times(1)
            .return_const(());

        let command = Command::new(Box::new(repo), Box::new(user_output));
        command.execute().expect("Failed to execute command");
    }

    #[test]
    fn test_month_week_ranges_2019_full_year() {
        let ranges = month_week_ranges(2019, 52);
        assert_eq!(ranges[0], (1, 5), "Jan 2019: weeks 1-5");
        assert_eq!(ranges[11], (49, 52), "Dec 2019: weeks 49-52");
    }

    #[test]
    fn test_month_week_ranges_2019_clamped() {
        let ranges = month_week_ranges(2019, 6);
        assert_eq!(ranges[0], (1, 5), "Jan 2019: weeks 1-5");
        assert_eq!(ranges[1], (6, 6), "Feb 2019: week 6 only");
        for entry in ranges.iter().skip(2) {
            assert!(entry.0 > entry.1, "months after clamp window are empty");
        }
    }

    #[test]
    fn test_month_header_no_collision_on_partial_year() {
        let header = month_header_line(2026, 26);
        assert!(
            !header.contains("JunJul"),
            "month abbreviations must not touch: {header:?}"
        );
        assert!(
            !header.contains("MayJun"),
            "month abbreviations must not touch: {header:?}"
        );
    }

    #[test]
    fn test_build_year_weekday_index_accumulates_kb() {
        let metadata = vec![
            EntryMetadata {
                timestamp: local_from("2026-06-21T16:50:46"),
                size: 5000,
            },
            EntryMetadata {
                timestamp: local_from("2026-06-21T17:00:00"),
                size: 3000,
            },
        ];
        let built = build_year_weekday_index(&metadata);
        let counts = built.get(&(Year(2026), Weekday::Sun)).unwrap();
        let week = NaiveDate::from_ymd_opt(2026, 6, 21)
            .unwrap()
            .iso_week()
            .week0() as usize;
        assert_eq!(counts[week], 8);
    }

    #[test]
    fn test_year_weekday_index_dec31_2024_at_end_of_calendar_year() {
        let metadata = vec![EntryMetadata {
            timestamp: local_from("2024-12-31T12:00:00"),
            size: 1024,
        }];
        let built = build_year_weekday_index(&metadata);
        assert!(
            !built.contains_key(&(Year(2025), Weekday::Tue)),
            "Dec 31 2024 must be grouped under its calendar year 2024, not ISO year 2025"
        );
        let counts = built
            .get(&(Year(2024), Weekday::Tue))
            .expect("Dec 31 2024 belongs to calendar year 2024");
        let last_idx = super::index::n_calendar_weeks(2024) - 1;
        assert_eq!(
            counts[last_idx], 1,
            "Dec 31 2024 must land in the last week of 2024 (index {last_idx}), not at the start"
        );
    }

    #[test]
    fn test_year_weekday_index_jan1_2023_at_start_of_calendar_year() {
        let metadata = vec![EntryMetadata {
            timestamp: local_from("2023-01-01T12:00:00"),
            size: 1024,
        }];
        let built = build_year_weekday_index(&metadata);
        assert!(
            !built.contains_key(&(Year(2022), Weekday::Sun)),
            "Jan 1 2023 must be grouped under its calendar year 2023, not ISO year 2022"
        );
        let counts = built
            .get(&(Year(2023), Weekday::Sun))
            .expect("Jan 1 2023 belongs to calendar year 2023");
        assert_eq!(
            counts[0], 1,
            "Jan 1 2023 must land in the first week of 2023 (index 0)"
        );
    }

    #[test]
    fn test_dec31_2025_appears_at_end_of_2025_not_in_2026() {
        let metadata = vec![EntryMetadata {
            timestamp: local_from("2025-12-31T12:00:00"),
            size: 1024,
        }];
        let built = build_year_weekday_index(&metadata);
        assert!(
            !built.contains_key(&(Year(2026), Weekday::Wed)),
            "Dec 31 2025 is ISO week 1 of 2026, but must be grouped under calendar year 2025"
        );
        let counts = built
            .get(&(Year(2025), Weekday::Wed))
            .expect("Dec 31 2025 belongs to calendar year 2025");
        let last_idx = super::index::n_calendar_weeks(2025) - 1;
        assert_eq!(
            counts[last_idx], 1,
            "Dec 31 2025 must land in the last week of 2025 (index {last_idx})"
        );
    }

    #[test]
    fn test_year_span_uses_calendar_year() {
        let metadata = vec![
            EntryMetadata {
                timestamp: local_from("2023-01-01T12:00:00"),
                size: 1024,
            },
            EntryMetadata {
                timestamp: local_from("2024-12-31T12:00:00"),
                size: 1024,
            },
        ];
        let (earliest, latest) = super::index::year_span(&metadata).unwrap();
        assert_eq!(earliest, 2023, "span uses calendar year of each entry");
        assert_eq!(latest, 2024, "span uses calendar year of each entry");
    }

    #[test]
    fn test_year_block_renders_full_calendar_week_width() {
        let metadata = vec![EntryMetadata {
            timestamp: local_from("2026-01-15T12:00:00"),
            size: 1024,
        }];
        let built = build_year_weekday_index(&metadata);
        let heatmap = Heatmap::new();
        let block = year_block(&built, &heatmap, 2026);
        let year_header = block.lines().next().expect("year block has a header line");
        let expected = super::index::n_calendar_weeks(2026) * 3;
        assert_eq!(
            year_header.len(),
            expected,
            "2026 must render full calendar-week width even though the only entry is in January"
        );
    }

    #[test]
    fn test_year_block_skips_year_with_no_entries() {
        let metadata = vec![EntryMetadata {
            timestamp: local_from("2026-06-15T12:00:00"),
            size: 1024,
        }];
        let built = build_year_weekday_index(&metadata);
        let heatmap = Heatmap::new();
        let block = year_block(&built, &heatmap, 2025);
        assert!(
            block.is_empty(),
            "a year with no entries must produce no output"
        );
    }

    #[test]
    fn test_cell_belongs_to_year_boundary_days() {
        use super::index::cell_belongs_to_year;
        let last = super::index::n_calendar_weeks(2025) - 1;
        assert!(
            cell_belongs_to_year(2025, last, Weekday::Wed),
            "Dec 31 2025 (Wed) is in 2025"
        );
        assert!(
            !cell_belongs_to_year(2025, last, Weekday::Thu),
            "Jan 1 2026 (Thu) is not in 2025"
        );
        assert!(
            !cell_belongs_to_year(2025, last, Weekday::Sun),
            "Jan 4 2026 (Sun) is not in 2025"
        );
        assert!(
            !cell_belongs_to_year(2025, 0, Weekday::Mon),
            "Dec 30 2024 (Mon) is not in 2025"
        );
        assert!(
            cell_belongs_to_year(2025, 0, Weekday::Wed),
            "Jan 1 2025 (Wed) is in 2025"
        );
    }

    #[test]
    fn test_last_used_week_missing_year_is_none() {
        let index: YearWeekdayIndex = std::collections::HashMap::new();
        assert!(last_used_week(&index, 2030).is_none());
    }

    #[test]
    fn test_stats_snapshot_multi_year() {
        console::set_colors_enabled(true);
        console::set_true_colors_enabled(true);
        let metadata = generate_multi_year_metadata();
        let built_index = build_year_weekday_index(&metadata);
        let (earliest_year, latest_year) = super::index::year_span(&metadata).unwrap();
        let heatmap = Heatmap::new();
        let mut result = String::new();
        for year in earliest_year..=latest_year {
            result += &year_block(&built_index, &heatmap, year);
        }
        insta::assert_snapshot!(result);
    }

    #[test]
    fn max_weeks_constant_matches_array_size() {
        assert_eq!(MAX_WEEKS, 60);
    }
}
