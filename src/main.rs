//! zellij-vertical-tab: renders the session's tabs vertically, one row per
//! tab, inside a fixed-width unselectable side pane.
//!
//! Interactions:
//! - left-click a row to switch to that tab
//! - scroll wheel moves the visible window when tabs overflow the pane height
//! - the active tab is always kept inside the visible window
//!
//! Row layout: `<lead><index> <name>` padded to the full pane width, where
//! `lead` is '▲' on the first visible row when tabs are hidden above, '▼' on
//! the last visible row when tabs are hidden below, and ' ' otherwise.
//! Styling comes from `Text`/`ztext` sequences, which Zellij renders with the
//! user's theme (active tab = `.selected()`).

use std::collections::BTreeMap;

use zellij_tile::prelude::*;

const ARROW_UP: char = '▲';
const ARROW_DOWN: char = '▼';

// `host_run_plugin_command` is normally provided by the Zellij wasm runtime.
// This stub lets host-target builds (`cargo test`, `cargo check`) link; it is
// never called because tests only exercise pure functions.
#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
extern "C" fn host_run_plugin_command() {}

#[derive(Default)]
struct State {
    tabs: Vec<TabInfo>,
    /// Index (into `tabs`) of the active tab.
    active_idx: Option<usize>,
    /// Index (into `tabs`) of the first visible row.
    scroll_offset: usize,
    /// Content height in rows, cached from the last `render`.
    rows: usize,
    /// Whether `set_selectable(false)` has been applied yet.
    unselectable_set: bool,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[EventType::TabUpdate, EventType::Mouse]);
        // NOTE: set_selectable(false) is NOT called here. On zellij 0.44,
        // calling it during initial session startup can kill the client when
        // the plugin pane lives in a default_tab_template; deferring it to the
        // first event (see update()) avoids that. Layouts should also wrap the
        // template's `children` in their own `pane { ... }` — see zellij.kdl.
    }

    fn update(&mut self, event: Event) -> bool {
        // Fixed-size plugin panes are only stable when unselectable (this is
        // the same pattern as the built-in tab-bar). Unselectable panes still
        // receive Mouse events.
        if !self.unselectable_set {
            self.unselectable_set = true;
            set_selectable(false);
        }
        match event {
            Event::TabUpdate(tabs) => {
                if tabs == self.tabs {
                    return false;
                }
                let new_active = tabs.iter().position(|t| t.active);
                self.tabs = tabs;
                if new_active != self.active_idx {
                    // The user switched tabs outside the plugin (keybindings,
                    // etc.): follow the active tab so it stays visible.
                    self.active_idx = new_active;
                    self.scroll_offset = visible_window(
                        self.tabs.len(),
                        self.active_idx,
                        self.scroll_offset,
                        self.rows,
                    );
                } else {
                    self.scroll_offset =
                        clamp_offset(self.tabs.len(), self.scroll_offset, self.rows);
                }
                true
            },
            Event::Mouse(mouse) => match mouse {
                Mouse::LeftClick(line, _col) => {
                    // Mouse coordinates are 0-based content cells; `line` is
                    // signed because it can go negative in scrollback (not
                    // possible here, but guard anyway).
                    if line >= 0 {
                        let idx = self.scroll_offset + line as usize;
                        if let Some(tab) = self.tabs.get(idx) {
                            // Tab indices are 1-based when switching.
                            switch_tab_to(tab.position as u32 + 1);
                        }
                    }
                    // The resulting TabUpdate triggers the re-render.
                    false
                },
                Mouse::ScrollUp(_) => {
                    let new_offset = self.scroll_offset.saturating_sub(1);
                    std::mem::replace(&mut self.scroll_offset, new_offset) != new_offset
                },
                Mouse::ScrollDown(_) => {
                    let new_offset =
                        clamp_offset(self.tabs.len(), self.scroll_offset + 1, self.rows);
                    std::mem::replace(&mut self.scroll_offset, new_offset) != new_offset
                },
                _ => false,
            },
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        if rows != self.rows {
            self.rows = rows;
            self.scroll_offset =
                visible_window(self.tabs.len(), self.active_idx, self.scroll_offset, rows);
        }
        if self.tabs.is_empty() || rows == 0 || cols == 0 {
            return;
        }

        let offset = self.scroll_offset;
        let visible_count = rows.min(self.tabs.len() - offset);
        // Keep indices right-aligned even when the count rolls past 9.
        let index_width = self.tabs.len().to_string().len();

        for i in 0..visible_count {
            let tab = &self.tabs[offset + i];
            let lead = if i == 0 && offset > 0 {
                ARROW_UP
            } else if i == visible_count - 1 && offset + visible_count < self.tabs.len() {
                ARROW_DOWN
            } else {
                ' '
            };
            let row = format_row(lead, tab.position + 1, index_width, &tab.name, cols);
            let text = Text::new(row);
            let text = if tab.active { text.selected() } else { text };
            print_text_with_coordinates(text, 0, i, Some(cols), Some(1));
        }
    }
}

/// Largest valid scroll offset: 0 when everything fits.
fn clamp_offset(tab_count: usize, offset: usize, rows: usize) -> usize {
    if rows == 0 || tab_count <= rows {
        0
    } else {
        offset.min(tab_count - rows)
    }
}

/// Scroll offset that keeps the active tab inside the `rows`-high window
/// while moving the previous offset as little as possible.
fn visible_window(tab_count: usize, active: Option<usize>, prev: usize, rows: usize) -> usize {
    let mut offset = clamp_offset(tab_count, prev, rows);
    if let (Some(active), true) = (active, rows > 0) {
        if active < offset {
            offset = active;
        } else if active >= offset + rows {
            offset = active + 1 - rows;
        }
    }
    offset
}

/// Build one tab row: `<lead><right-aligned index> <name>` fitted to `width`.
fn format_row(lead: char, index: usize, index_width: usize, name: &str, width: usize) -> String {
    let body = format!("{}{:>iw$} {}", lead, index, name, iw = index_width);
    fit_to_width(&body, width)
}

/// Truncate to `width` chars and pad with spaces so row-wide styles (e.g.
/// selected) span the whole row. Zellij additionally clips display overflow,
/// so wide characters in `name` are handled at render time.
fn fit_to_width(s: &str, width: usize) -> String {
    let mut out: String = s.chars().take(width).collect();
    let len = out.chars().count();
    if len < width {
        out.push_str(&" ".repeat(width - len));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_no_overflow() {
        assert_eq!(visible_window(5, Some(2), 0, 10), 0);
    }

    #[test]
    fn window_zero_rows() {
        assert_eq!(visible_window(5, Some(2), 3, 0), 0);
    }

    #[test]
    fn window_no_active() {
        assert_eq!(visible_window(30, None, 25, 10), 20);
    }

    #[test]
    fn window_follows_active_down() {
        assert_eq!(visible_window(30, Some(15), 0, 10), 6);
    }

    #[test]
    fn window_follows_active_up() {
        assert_eq!(visible_window(30, Some(3), 10, 10), 3);
    }

    #[test]
    fn window_keeps_offset_when_active_visible() {
        assert_eq!(visible_window(30, Some(12), 10, 10), 10);
    }

    #[test]
    fn window_clamps_when_tabs_shrink() {
        assert_eq!(visible_window(11, Some(10), 9, 10), 1);
    }

    #[test]
    fn clamp_basic() {
        assert_eq!(clamp_offset(30, 25, 10), 20);
        assert_eq!(clamp_offset(5, 3, 10), 0);
        assert_eq!(clamp_offset(5, 3, 0), 0);
    }

    #[test]
    fn fit_pads_short_rows() {
        assert_eq!(fit_to_width("ab", 4), "ab  ");
    }

    #[test]
    fn fit_truncates_long_rows() {
        assert_eq!(fit_to_width("abcdef", 3), "abc");
    }

    #[test]
    fn row_format_pads_to_width() {
        assert_eq!(format_row(' ', 3, 1, "work", 10), " 3 work   ");
    }

    #[test]
    fn row_format_aligns_double_digit_indices() {
        assert_eq!(format_row(ARROW_DOWN, 10, 2, "x", 8), "▼10 x   ");
        assert_eq!(format_row(ARROW_UP, 9, 2, "x", 8), "▲ 9 x   ");
    }
}
