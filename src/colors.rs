use iced::Color;

pub const BG_PRIMARY: Color = rgb(1, 1, 1);
pub const BORDER_PRIMARY: Color = rgb(60, 8, 100);
pub const TEXT_PRIMARY: Color = rgb(230, 230, 230);
pub const TEXT_SECONDARY: Color = rgb(150, 4, 250);

const fn rgb(r: u8, g: u8, b: u8) -> Color {
	Color::from_rgb8(r, g, b)
}

const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
	Color::from_rgba8(r, g, b, a)
}

pub const SHADOW_PRIMARY: Color = rgba(60, 8, 100, 0.28);
