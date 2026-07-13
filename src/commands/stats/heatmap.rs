use colorgrad::{Color, Gradient as _};
use dialoguer::console;

const COLOR_TITAN_WHITE: Color = Color::from_rgba8(0xe5, 0xe8, 0xff, 0x00);
const COLOR_MELROSE: Color = Color::from_rgba8(0x91, 0x9b, 0xff, 0x00);
const COLOR_TOREA_BAY: Color = Color::from_rgba8(0x13, 0x3a, 0x94, 0x00);
const COLOR_WILD_STRAWBERRY: Color = Color::from_rgba8(0xff, 0x40, 0x7e, 0x00);
const COLOR_EMPTY: Color = Color::from_rgba8(0x33, 0x33, 0x33, 0x00);

const DISPLAY_FLOOR: u32 = 10;

pub struct Heatmap {
    gradient: colorgrad::LinearGradient,
}

impl Heatmap {
    pub fn new() -> Self {
        let gradient = colorgrad::GradientBuilder::new()
            .colors(&[
                COLOR_TITAN_WHITE,
                COLOR_MELROSE,
                COLOR_TOREA_BAY,
                COLOR_WILD_STRAWBERRY,
            ])
            .domain(&[0.0, 1.0, 3.0, DISPLAY_FLOOR as f32])
            .build::<colorgrad::LinearGradient>()
            .expect("hardcoded gradient is valid");
        Self { gradient }
    }

    pub fn cell(&self, count: u32) -> Option<(console::Color, console::Color, String)> {
        if count == 0 {
            return None;
        }
        let capped = count.min(DISPLAY_FLOOR) as f32;
        let bg_color = self.gradient.at(capped);
        let bg = to_console_color(&bg_color);
        let fg = pick_foreground(&bg_color);
        let text = if count > DISPLAY_FLOOR - 1 {
            " >9".to_string()
        } else {
            format!("  {count}")
        };
        Some((bg, fg, text))
    }

    pub const fn empty_cell_bg() -> console::Color {
        to_console_color(&COLOR_EMPTY)
    }
}

impl Default for Heatmap {
    fn default() -> Self {
        Self::new()
    }
}

const fn to_console_color(color: &colorgrad::Color) -> console::Color {
    let [r, g, b, _] = color.to_rgba8();
    console::Color::TrueColor(r, g, b)
}

fn relative_luminance(color: &colorgrad::Color) -> f32 {
    let [r, g, b, _] = color.to_linear_rgba();
    0.0722f32.mul_add(b, 0.7152f32.mul_add(g, 0.2126 * r))
}

fn michelson_contrast(color_1: &colorgrad::Color, color_2: &colorgrad::Color) -> f32 {
    let l1 = relative_luminance(color_1);
    let l2 = relative_luminance(color_2);
    (l2 - l1) * (l2 + l1)
}

fn pick_foreground(background: &colorgrad::Color) -> console::Color {
    let candidates = [Color::new(0., 0., 0., 1.), Color::new(1., 1., 1., 1.)];
    let chosen = candidates
        .iter()
        .max_by(|x, y| {
            michelson_contrast(x, background).total_cmp(&michelson_contrast(y, background))
        })
        .expect("non-empty candidate set");
    to_console_color(chosen)
}
