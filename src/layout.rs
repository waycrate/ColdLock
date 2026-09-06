use iced::advanced::text::{self, Paragraph, Wrapping};
use iced::{Font, Size};

pub(crate) const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};
const SCREEN_MARGIN: f32 = 16.0;
pub(crate) const LINE_HEIGHT: f32 = 1.3;
pub(crate) const DETAILS_PADDING: f32 = 12.0;
const CLOCK_GAP: f32 = 70.0;
const AVATAR_SIZE: f32 = 120.0;
const PREFERRED_MIN_SCALE: f32 = 0.5;
const ERROR_SIZE: f32 = 16.0;
pub(crate) const WELCOME_HINT: &str = "Press Enter to unlock";

pub(crate) struct UserSizes {
    pub welcome_name_size: f32,
    pub auth_name_size: f32,
    pub hint_size: f32,
    pub input_size: f32,
    pub input_padding: f32,
    pub input_width: f32,
    pub error_size: f32,
    pub welcome_gap: f32,
    pub hint_gap: f32,
    pub auth_gap: f32,
}

impl UserSizes {
    pub fn new(scale: f32) -> Self {
        Self {
            welcome_name_size: 35.0 * scale,
            auth_name_size: 45.0 * scale,
            hint_size: 22.0 * scale,
            input_size: 30.0 * scale,
            input_padding: 10.0 * scale,
            input_width: 320.0 * scale,
            error_size: ERROR_SIZE * scale,
            welcome_gap: 5.0 * scale,
            hint_gap: 40.0 * scale,
            auth_gap: 40.0 * scale,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ScreenText<'a> {
    Welcome {
        name: &'a str,
        time: &'a str,
        date: &'a str,
    },
    Auth {
        name: &'a str,
        error: &'a str,
    },
}

impl ScreenText<'_> {
    // text and controls below avatar
    fn details_height(&self, scale: f32, width: f32) -> f32 {
        let sizes = UserSizes::new(scale);
        let text_height = |content, size, font| {
            measure_text(content, size, font, width, Wrapping::WordOrGlyph).height
        };
        match *self {
            Self::Welcome { name, .. } => {
                sizes.welcome_gap
                    + text_height(name, sizes.welcome_name_size, Font::DEFAULT)
                    + sizes.hint_gap
                    + text_height(WELCOME_HINT, sizes.hint_size, Font::DEFAULT)
            }
            Self::Auth { name, error } => {
                let error_height = if error.is_empty() {
                    0.0
                } else {
                    sizes.auth_gap + text_height(error, sizes.error_size, Font::DEFAULT)
                };
                2.0 * sizes.auth_gap
                    + text_height(name, sizes.auth_name_size, BOLD)
                    + sizes.input_size * LINE_HEIGHT
                    + 2.0 * sizes.input_padding
                    + error_height
            }
        }
    }
}

pub(crate) fn measure_text(
    content: &str,
    font_size: f32,
    font: Font,
    max_width: f32,
    wrapping: Wrapping,
) -> Size {
    type TextParagraph = <iced::Renderer as text::Renderer>::Paragraph;
    TextParagraph::with_text(text::Text {
        content,
        bounds: Size::new(max_width, f32::INFINITY),
        size: font_size.into(),
        font,
        line_height: text::LineHeight::Relative(LINE_HEIGHT),
        align_x: text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: text::Shaping::default(),
        wrapping,
    })
    .min_bounds()
}

fn side_margin(clock_width: f32) -> f32 {
    (clock_width * 0.25).max(SCREEN_MARGIN)
}

struct ClockText<'a> {
    time: &'a str,
    date: &'a str,
    unscaled_size: Size,
}

impl<'a> ClockText<'a> {
    fn new(time: &'a str, date: &'a str) -> Self {
        let mut clock = Self {
            time,
            date,
            unscaled_size: Size::ZERO,
        };
        clock.unscaled_size = clock.measure(1.0);
        clock
    }

    fn measure(&self, scale: f32) -> Size {
        let time = measure_text(self.time, 75.0 * scale, BOLD, f32::INFINITY, Wrapping::None);
        let date = measure_text(self.date, 35.0 * scale, BOLD, f32::INFINITY, Wrapping::None);
        Size::new(
            time.width.max(date.width),
            time.height + date.height + 5.0 * scale,
        )
    }
}

pub(crate) fn fit_layout(surface: Size, content: ScreenText<'_>) -> LayoutMetrics {
    let clock = match content {
        ScreenText::Welcome { time, date, .. } => Some(ClockText::new(time, date)),
        ScreenText::Auth { .. } => None,
    };
    let available_width = surface.width - 2.0 * SCREEN_MARGIN;
    let half_height = surface.height / 2.0 - SCREEN_MARGIN;

    let width_scale = match &clock {
        Some(clock) => {
            let max_clock_width = surface.width / 1.5;
            (max_clock_width / clock.unscaled_size.width).min(available_width / AVATAR_SIZE)
        }
        None => available_width / AVATAR_SIZE,
    };

    let width_fit_scale = width_scale.min(1.0);
    let margin = match &clock {
        Some(clock) => side_margin(clock.measure(width_fit_scale).width),
        None => SCREEN_MARGIN,
    };
    let details_width = surface.width - 2.0 * margin - 2.0 * DETAILS_PADDING;
    let unscaled_details_height = content.details_height(1.0, details_width);

    let (height_scale, fixed_content_scale) = match &clock {
        Some(clock) => {
            let top_scale =
                half_height / (clock.unscaled_size.height + CLOCK_GAP + AVATAR_SIZE / 2.0);
            let bottom_scale = half_height / (AVATAR_SIZE / 2.0 + unscaled_details_height);
            (top_scale.min(bottom_scale), top_scale)
        }
        None => {
            let available_height = surface.height - 2.0 * SCREEN_MARGIN;
            (
                available_height / (AVATAR_SIZE + unscaled_details_height),
                available_height / AVATAR_SIZE,
            )
        }
    };

    let scale = height_scale
        .max(PREFERRED_MIN_SCALE)
        .min(width_scale)
        .min(fixed_content_scale)
        .min(1.0);

    let avatar_size = AVATAR_SIZE * scale;
    let clock_size = clock.as_ref().map(|clock| clock.measure(scale));
    let margin = match clock_size {
        Some(size) => side_margin(size.width),
        None => SCREEN_MARGIN,
    };
    let mut metrics = LayoutMetrics {
        scale,
        clock_top: 0.0,
        avatar_top: (surface.height - avatar_size).max(0.0) / 2.0,
        avatar_size,
        foreground_width: surface.width - 2.0 * margin,
        details_height: (surface.height / 2.0 - avatar_size / 2.0 - SCREEN_MARGIN).max(0.0),
    };

    match clock_size {
        Some(size) => {
            metrics.clock_top = (metrics.avatar_top - CLOCK_GAP * scale - size.height).max(0.0);
        }
        None => {
            // Centre avatar and form together
            let details_height = content.details_height(scale, metrics.details_width());
            metrics.details_height =
                details_height.min((surface.height - 2.0 * SCREEN_MARGIN - avatar_size).max(0.0));
            metrics.avatar_top =
                ((surface.height - avatar_size - metrics.details_height) / 2.0).max(0.0);
        }
    }
    metrics
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutMetrics {
    pub scale: f32,
    pub clock_top: f32,
    pub avatar_top: f32,
    pub avatar_size: f32,
    pub foreground_width: f32,
    pub details_height: f32,
}

impl LayoutMetrics {
    pub fn details_width(&self) -> f32 {
        self.foreground_width - 2.0 * DETAILS_PADDING
    }
}
