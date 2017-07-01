#![warn(missing_docs)]

use imgui_sys;
use std::ptr;
use super::{ImStr, ImVec2};


/// Progress bar widget.
#[must_use]
pub struct ProgressBar<'p> {
    fraction: f32,
    size: ImVec2,
    overlay_text: Option<&'p ImStr>,
}

impl<'p> ProgressBar<'p> {
    /// Creates a progress bar with a given fraction showing
    /// the progress (0.0 = 0%, 1.0 = 100%).
    /// The progress bar will be automatically sized to fill
    /// the entire width of the window if no custom size is
    /// specified.
    pub fn new(fraction: f32) -> Self {
        ProgressBar {
            fraction: fraction,
            size: ImVec2::new(-1.0, 0.0),
            overlay_text: None,
        }
    }

    /// Sets an optional text that will be drawn over the progress bar.
    #[inline]
    pub fn overlay_text(mut self, overlay_text: &'p ImStr) -> Self {
        self.overlay_text = Some(overlay_text);
        self
    }

    /// Sets the size of the progress bar. Negative values will automatically
    /// align to the end of the axis, zero will let the progress bar choose a
    /// size and positive values will use the given size.
    #[inline]
    pub fn size(mut self, size: ImVec2) -> Self {
        self.size = size;
        self
    }

    /// Builds the progress bar. This has to be called after setting all parameters
    /// of the progress bar, otherwise the it will not be shown.
    pub fn build(self) {
        unsafe {
            imgui_sys::igProgressBar(self.fraction,
                                     &self.size,
                                     self.overlay_text
                                         .map(|x| x.as_ptr())
                                         .unwrap_or(ptr::null()));
        }
    }
}
