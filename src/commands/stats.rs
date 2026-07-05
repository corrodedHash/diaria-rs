
use chrono::{Datelike as _, Weekday};
use colorgrad::{Color, Gradient as _};
use dialoguer::console;

use crate::entry::repository::{DiariaEntryRepository, DiariaFsRepository, EntryMetadata};
use crate::stdout_printer::{RealUserOutput, UserOutput};

pub struct Command<T: DiariaEntryRepository, PRINT: UserOutput> {
    repository: T,
    stdout_printer: PRINT,
}

impl<T: DiariaEntryRepository, PRINT: UserOutput> Command<T, PRINT> {
    pub fn new(repository: T, stdout_printer: PRINT) -> Self {
        Self {
            repository,
            stdout_printer,
        }
    }
}

impl Default for Command<DiariaFsRepository, RealUserOutput> {
    fn default() -> Self {
        Self {
            repository: DiariaFsRepository {},
            stdout_printer: RealUserOutput {},
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Year(u32);

type YearWeekdayIndex = std::collections::HashMap<(Year, chrono::Weekday), [u32; 60]>;

use chrono::{Datelike, NaiveDate};

fn get_weekday_range(year: Year, target_weekday: chrono::Weekday) -> (u32, u32) {
    // Get their weekdays and the week of December 31
    let jan_1_weekday = NaiveDate::from_ymd_opt(year.0 as i32, 1, 1)
        .unwrap()
        .weekday();
    let dec_31_weekday = NaiveDate::from_ymd_opt(year.0 as i32, 12, 31)
        .unwrap()
        .weekday();
    let dec_31_week = NaiveDate::from_ymd_opt(year.0 as i32, 12, 31)
        .unwrap()
        .iso_week()
        .week();

    // Calculate the minimum week for the target weekday
    let min_week = if target_weekday.number_from_monday() >= jan_1_weekday.number_from_monday() {
        1
    } else {
        2
    };
    // Calculate the maximum week for the target weekday
    let max_week = if target_weekday.number_from_monday() <= dec_31_weekday.number_from_monday() {
        dec_31_week
    } else {
        dec_31_week - 1
    };

    (min_week, max_week)
}

fn relative_luminance(color: &colorgrad::Color) -> f32 {
    let [r, g, b, _] = color.to_linear_rgba();
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn michelson_contrast(color_1: &colorgrad::Color, color_2: &colorgrad::Color) -> f32 {
    let l1 = relative_luminance(color_1);
    let l2 = relative_luminance(color_2);
    (l2 - l1) * (l2 + l1)
}

fn to_console_color(color: &colorgrad::Color) -> console::Color {
    let [r, g, b, _] = color.to_rgba8();
    console::Color::TrueColor(r, g, b)
}

fn year_weekday_line(counts: &YearWeekdayIndex, year: Year, weekday: chrono::Weekday) -> String {
    let line = counts.get(&(year, weekday)).unwrap_or(&[0u32; 60]);
    let mut output = "".to_owned();
    let (min_kw, max_kw) = get_weekday_range(year, weekday);
    const COLOR_ATLANTIS: Color = Color::from_rgba8(0x5a, 0xd5, 0x2d, 0x00);
    const COLOR_TITAN_WHITE: Color = Color::from_rgba8(0xe5, 0xe8, 0xff, 0x00);
    const COLOR_MELROSE: Color = Color::from_rgba8(0x91, 0x9b, 0xff, 0x00);
    const COLOR_TOREA_BAY: Color = Color::from_rgba8(0x13, 0x3a, 0x94, 0x00);
    const COLOR_WILD_STRAWBERRY: Color = Color::from_rgba8(0xff, 0x40, 0x7e, 0x00);

    let g = colorgrad::GradientBuilder::new()
        .colors(&[
            // COLOR_ATLANTIS,
            COLOR_TITAN_WHITE,
            COLOR_MELROSE,
            COLOR_TOREA_BAY,
            COLOR_WILD_STRAWBERRY,
        ])
        .domain(&[0.0, 500.0, 4000.0, 12000.0])
        .build::<colorgrad::LinearGradient>()
        .expect("Should work");

    for (index, size) in line.iter().enumerate() {
        if (index as u32) < min_kw || (index as u32) > max_kw {
            output += "   ";
            continue;
        }
        let chosen_color = g.at(*size as f32);
        let console_color = to_console_color(&chosen_color);
        let fg_color_choices = [
            colorgrad::Color::new(0., 0., 0., 1.), // black
            colorgrad::Color::new(1., 1., 1., 1.), //white
        ];
        let fg_color = fg_color_choices
            .iter()
            .max_by(move |x, y| {
                michelson_contrast(x, &chosen_color)
                    .total_cmp(&michelson_contrast(y, &chosen_color))
            })
            .unwrap();

        let size_kb = ((*size as f32) / 1024.0).round() as u8;
        let formatted_size = if size_kb > 9 {
            " >9"
        } else {
            &format!("  {}", size_kb)
        };
        output += &format!(
            "{}",
            console::style(formatted_size)
                .bg(console_color)
                .fg(to_console_color(fg_color))
        );
    }

    output
}

// fn year_header(year: Year) -> String {

//     let abbr = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

//         let months: Vec<(Month, &str)> = (1..=12)
//             .map(|num| {
//                 let month = Month::try_from(num).unwrap();
//                 (month, month.())
//             })
//             .collect();

//         for (month, abbr) in months {
//             println!("{:?}: {}", month, abbr);
//         }
// }

fn build_year_weekday_index(metadata: &[EntryMetadata]) -> YearWeekdayIndex {
    let mut counts: YearWeekdayIndex = std::collections::HashMap::new();

    for entry in metadata {
        let year = entry.timestamp.year();
        let week = entry.timestamp.iso_week().week0();
        let weekday = entry.timestamp.weekday();
        *counts
            .entry((Year(year as u32), weekday))
            .or_insert([0u32; 60])
            .get_mut(week as usize)
            .unwrap() += entry.size as u32;
    }

    counts
}

impl<T: DiariaEntryRepository, PRINT: UserOutput> Command<T, PRINT> {
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = self.repository.list_entry_metadata();
        if metadata.is_empty() {
            return Ok(());
        }
        let earliest_year = metadata.iter().map(|x| x.timestamp.year()).min().unwrap();
        let latest_year = metadata.iter().map(|x| x.timestamp.year()).max().unwrap();

        let index = build_year_weekday_index(&metadata);
        let mut result = String::new();
        for year in earliest_year..=latest_year {
            for weekday in &[
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
                Weekday::Sun,
            ] {
                result += &year_weekday_line(&index, Year(year as u32), *weekday);
                result += "\n";
            }
        }

        console::set_colors_enabled(true);
        console::set_true_colors_enabled(true);
        self.stdout_printer.print(&result);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use crate::entry::repository::{EntryMetadata, MockDiariaEntryRepository};
    use crate::stdout_printer::MockUserOutput;

    use super::*;

    #[test]
    fn test_stats_execute_with_mocked_entries() {
        let mut repo = MockDiariaEntryRepository::new();
        let metadata = vec![
            EntryMetadata {
                id: "1".to_string(),
                timestamp: chrono::Local
                    .from_local_datetime(
                        &chrono::NaiveDateTime::parse_from_str(
                            "2026-06-21T16:50:46",
                            "%Y-%m-%dT%H:%M:%S",
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                size: 1024,
            },
            EntryMetadata {
                id: "2".to_string(),
                timestamp: chrono::Local
                    .from_local_datetime(
                        &chrono::NaiveDateTime::parse_from_str(
                            "2026-06-22T16:50:46",
                            "%Y-%m-%dT%H:%M:%S",
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                size: 2048,
            },
        ];
        repo.expect_list_entry_metadata()
            .return_once(move || metadata);

        // let mut user_output = MockUserOutput::new();
        // user_output
        //     .expect_print()
        //     .withf(|text| text.contains("  1"))
        //     .return_const(());

        let command = Command::new(repo, RealUserOutput {});
        command.execute().expect("Failed to execute command");
    }
}
