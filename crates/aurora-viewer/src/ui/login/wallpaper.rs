//! Login artwork is independent from loading artwork and editable before login.
use crate::theme::Palette;
use egui::{Context, Painter, Rect, Shape};
#[derive(Default)]
pub struct Wallpaper;
impl Wallpaper {
    pub fn paint(&mut self, ctx: &Context, painter: &Painter, screen: Rect, p: &Palette) -> bool {
        crate::ui::branding_runtime::paint_background(ctx, painter, screen, p, false)
    }
}
pub fn frosted_shape(ctx: &Context, screen: Rect, pane: Rect, radius: egui::CornerRadius) -> Option<Shape> {
    crate::ui::branding_runtime::frosted_shape(ctx, screen, pane, radius)
}
