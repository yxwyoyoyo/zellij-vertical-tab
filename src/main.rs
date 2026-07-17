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

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;
use zellij_tile::prelude::*;

const ARROW_UP: char = '▲';
const ARROW_DOWN: char = '▼';
const AGENT_STATUS_PIPE: &str = "vertical-tab-agent-status";
const AGENT_STATUS_SYNC_UPDATE: &str = "vertical-tab-agent-status-sync-update";
const AGENT_STATUS_SYNC_REQUEST: &str = "vertical-tab-agent-status-sync-request";
const AGENT_STATUS_SYNC_SNAPSHOT: &str = "vertical-tab-agent-status-sync-snapshot";
const AGENT_STATUS_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AgentState {
    Idle,
    Working,
    Waiting,
    Done,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BadgeColor {
    Dim,
    Emphasis(usize),
    Success,
    None,
}

impl AgentState {
    fn glyph(self) -> &'static str {
        // These Font Awesome glyphs are native to Nerd Font Mono builds. They
        // share identical vertical metrics, avoiding the baseline mismatch
        // caused when geometric Unicode symbols come from a fallback font.
        match self {
            Self::Idle => "",
            Self::Working => "",
            Self::Waiting => "",
            Self::Done => "",
            Self::Clear => "",
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::Waiting => 4,
            Self::Working => 3,
            Self::Done => 2,
            Self::Idle => 1,
            Self::Clear => 0,
        }
    }

    fn badge_color(self) -> BadgeColor {
        match self {
            Self::Idle => BadgeColor::Dim,
            Self::Working => BadgeColor::Emphasis(1), // theme cyan
            Self::Waiting => BadgeColor::Emphasis(0), // theme orange
            Self::Done => BadgeColor::Success,        // theme green
            Self::Clear => BadgeColor::None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AgentStatusPayload {
    version: u8,
    pane_id: String,
    session_id: String,
    state: AgentState,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct AgentStatusSnapshot {
    version: u8,
    records: Vec<AgentStatusPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AgentRecord {
    session_id: String,
    state: AgentState,
    updated_at_ms: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct AgentStatusUpdate {
    pane_id: u32,
    record: AgentRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TabAgentSummary {
    state: AgentState,
    count: usize,
}

impl TabAgentSummary {
    fn badge(self) -> String {
        if self.count > 1 {
            format!("{}{}", self.state.glyph(), self.count)
        } else {
            self.state.glyph().to_owned()
        }
    }
}

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
    /// Current tab position for every terminal pane reported by Zellij.
    pane_tabs: HashMap<u32, usize>,
    /// Most recent top-level Codex session status for each terminal pane.
    agent_records: HashMap<u32, AgentRecord>,
    /// This sidebar instance's session-unique Zellij plugin ID.
    plugin_id: Option<u32>,
    /// Other sidebar plugin instances discovered across the session's tabs.
    peer_plugin_ids: HashSet<u32>,
    /// Whether `set_selectable(false)` has been applied yet.
    unselectable_set: bool,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.plugin_id = Some(get_plugin_ids().plugin_id);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::ReadCliPipes,
            PermissionType::MessageAndLaunchOtherPlugins,
        ]);
        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::Mouse,
        ]);
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
            }
            Event::PaneUpdate(pane_manifest) => {
                let pane_tabs = terminal_pane_tabs(&pane_manifest);
                let records_removed =
                    remove_missing_agent_records(&mut self.agent_records, &pane_tabs);
                let panes_changed = pane_tabs != self.pane_tabs;
                self.pane_tabs = pane_tabs;
                let peers = self
                    .plugin_id
                    .map(|plugin_id| sidebar_plugin_peers(&pane_manifest, plugin_id))
                    .unwrap_or_default();
                for peer_id in peers.difference(&self.peer_plugin_ids) {
                    if let Some(plugin_id) = self.plugin_id {
                        send_plugin_message(
                            *peer_id,
                            AGENT_STATUS_SYNC_REQUEST,
                            plugin_id.to_string(),
                        );
                    }
                }
                let peers_changed = peers != self.peer_plugin_ids;
                self.peer_plugin_ids = peers;
                records_removed || panes_changed || peers_changed
            }
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
                }
                Mouse::ScrollUp(_) => {
                    let new_offset = self.scroll_offset.saturating_sub(1);
                    std::mem::replace(&mut self.scroll_offset, new_offset) != new_offset
                }
                Mouse::ScrollDown(_) => {
                    let new_offset =
                        clamp_offset(self.tabs.len(), self.scroll_offset + 1, self.rows);
                    std::mem::replace(&mut self.scroll_offset, new_offset) != new_offset
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let Some(payload) = pipe_message.payload.as_deref() else {
            return false;
        };
        match pipe_message.name.as_str() {
            AGENT_STATUS_PIPE => {
                let Some(update) = parse_agent_status(payload) else {
                    return false;
                };
                let changed = apply_agent_status(&mut self.agent_records, update);
                for peer_id in &self.peer_plugin_ids {
                    send_plugin_message(*peer_id, AGENT_STATUS_SYNC_UPDATE, payload.to_owned());
                }
                changed
            }
            AGENT_STATUS_SYNC_UPDATE => parse_agent_status(payload)
                .is_some_and(|update| apply_agent_status(&mut self.agent_records, update)),
            AGENT_STATUS_SYNC_REQUEST => {
                let Ok(requester_id) = payload.parse::<u32>() else {
                    return false;
                };
                if Some(requester_id) == self.plugin_id {
                    return false;
                }
                if let Some(snapshot) = serialize_agent_snapshot(&self.agent_records) {
                    send_plugin_message(requester_id, AGENT_STATUS_SYNC_SNAPSHOT, snapshot);
                }
                false
            }
            AGENT_STATUS_SYNC_SNAPSHOT => apply_agent_snapshot(&mut self.agent_records, payload),
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
        let summaries = aggregate_agent_statuses(&self.agent_records, &self.pane_tabs);

        for i in 0..visible_count {
            let tab = &self.tabs[offset + i];
            let lead = if i == 0 && offset > 0 {
                ARROW_UP
            } else if i == visible_count - 1 && offset + visible_count < self.tabs.len() {
                ARROW_DOWN
            } else {
                ' '
            };
            let summary = summaries.get(&tab.position);
            let badge = summary.map(|summary| summary.badge());
            let row = format_row(
                lead,
                tab.position + 1,
                index_width,
                &tab.name,
                badge.as_deref(),
                cols,
            );
            let mut text = Text::new(row);
            if let (Some(summary), Some(badge)) = (summary, badge.as_deref()) {
                text = color_agent_badge(text, summary.state, badge.chars().count());
            }
            let text = if tab.active { text.selected() } else { text };
            print_text_with_coordinates(text, 0, i, Some(cols), Some(1));
        }
    }
}

fn parse_terminal_pane_id(value: &str) -> Option<u32> {
    let digits = value.strip_prefix("terminal_").unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn parse_agent_status(payload: &str) -> Option<AgentStatusUpdate> {
    let payload: AgentStatusPayload = serde_json::from_str(payload).ok()?;
    if payload.version != AGENT_STATUS_VERSION || payload.session_id.trim().is_empty() {
        return None;
    }
    Some(AgentStatusUpdate {
        pane_id: parse_terminal_pane_id(&payload.pane_id)?,
        record: AgentRecord {
            session_id: payload.session_id,
            state: payload.state,
            updated_at_ms: payload.updated_at_ms,
        },
    })
}

fn serialize_agent_snapshot(records: &HashMap<u32, AgentRecord>) -> Option<String> {
    let mut records = records
        .iter()
        .map(|(pane_id, record)| AgentStatusPayload {
            version: AGENT_STATUS_VERSION,
            pane_id: format!("terminal_{pane_id}"),
            session_id: record.session_id.clone(),
            state: record.state,
            updated_at_ms: record.updated_at_ms,
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.pane_id.cmp(&right.pane_id));
    serde_json::to_string(&AgentStatusSnapshot {
        version: AGENT_STATUS_VERSION,
        records,
    })
    .ok()
}

fn apply_agent_snapshot(records: &mut HashMap<u32, AgentRecord>, payload: &str) -> bool {
    let Ok(snapshot) = serde_json::from_str::<AgentStatusSnapshot>(payload) else {
        return false;
    };
    if snapshot.version != AGENT_STATUS_VERSION {
        return false;
    }
    snapshot.records.into_iter().fold(false, |changed, record| {
        let update = serde_json::to_string(&record)
            .ok()
            .and_then(|record| parse_agent_status(&record));
        update
            .map(|update| apply_agent_status(records, update) || changed)
            .unwrap_or(changed)
    })
}

fn send_plugin_message(destination_plugin_id: u32, name: &str, payload: String) {
    pipe_message_to_plugin(
        MessageToPlugin::new(name)
            .with_destination_plugin_id(destination_plugin_id)
            .with_payload(payload),
    );
}

fn apply_agent_status(records: &mut HashMap<u32, AgentRecord>, update: AgentStatusUpdate) -> bool {
    if records
        .get(&update.pane_id)
        .is_some_and(|current| update.record.updated_at_ms < current.updated_at_ms)
    {
        return false;
    }

    if update.record.state == AgentState::Clear
        && records
            .get(&update.pane_id)
            .is_some_and(|current| current.session_id != update.record.session_id)
    {
        return false;
    }

    if records.get(&update.pane_id) == Some(&update.record) {
        return false;
    }
    records.insert(update.pane_id, update.record);
    true
}

fn terminal_pane_tabs(pane_manifest: &PaneManifest) -> HashMap<u32, usize> {
    pane_manifest
        .panes
        .iter()
        .flat_map(|(tab_position, panes)| {
            panes
                .iter()
                .filter(|pane| !pane.is_plugin)
                .map(|pane| (pane.id, *tab_position))
        })
        .collect()
}

fn sidebar_plugin_peers(pane_manifest: &PaneManifest, plugin_id: u32) -> HashSet<u32> {
    let plugin_url = pane_manifest
        .panes
        .values()
        .flatten()
        .find(|pane| pane.is_plugin && pane.id == plugin_id)
        .and_then(|pane| pane.plugin_url.as_deref());
    let Some(plugin_url) = plugin_url else {
        return HashSet::new();
    };
    pane_manifest
        .panes
        .values()
        .flatten()
        .filter(|pane| {
            pane.is_plugin && pane.id != plugin_id && pane.plugin_url.as_deref() == Some(plugin_url)
        })
        .map(|pane| pane.id)
        .collect()
}

fn remove_missing_agent_records(
    records: &mut HashMap<u32, AgentRecord>,
    pane_tabs: &HashMap<u32, usize>,
) -> bool {
    let old_count = records.len();
    records.retain(|pane_id, _| pane_tabs.contains_key(pane_id));
    old_count != records.len()
}

fn aggregate_agent_statuses(
    records: &HashMap<u32, AgentRecord>,
    pane_tabs: &HashMap<u32, usize>,
) -> HashMap<usize, TabAgentSummary> {
    let mut summaries: HashMap<usize, TabAgentSummary> = HashMap::new();
    for (pane_id, record) in records {
        if record.state == AgentState::Clear {
            continue;
        }
        let Some(tab_position) = pane_tabs.get(pane_id) else {
            continue;
        };
        summaries
            .entry(*tab_position)
            .and_modify(|summary| {
                summary.count += 1;
                if record.state.priority() > summary.state.priority() {
                    summary.state = record.state;
                }
            })
            .or_insert(TabAgentSummary {
                state: record.state,
                count: 1,
            });
    }
    summaries
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

/// Build one tab row with an optional right-aligned status badge.
fn format_row(
    lead: char,
    index: usize,
    index_width: usize,
    name: &str,
    badge: Option<&str>,
    width: usize,
) -> String {
    let prefix = format!("{}{:>iw$} ", lead, index, iw = index_width);
    let Some(badge) = badge.filter(|badge| !badge.is_empty()) else {
        return fit_tab_body(&prefix, name, width);
    };
    let badge_width = display_width(badge);
    if badge_width >= width {
        return fit_to_width(badge, width);
    }
    let body_width = width - badge_width - 1;
    format!("{} {}", fit_tab_body(&prefix, name, body_width), badge)
}

fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|character| character.width().unwrap_or(0))
        .sum()
}

fn color_agent_badge(text: Text, state: AgentState, badge_chars: usize) -> Text {
    let badge_start = text.len().saturating_sub(badge_chars);
    match state.badge_color() {
        BadgeColor::Dim => text.dim_range(badge_start..),
        BadgeColor::Emphasis(level) => text.color_range(level, badge_start..),
        BadgeColor::Success => text.success_color_range(badge_start..),
        BadgeColor::None => text,
    }
}

fn fit_tab_body(prefix: &str, name: &str, width: usize) -> String {
    let prefix_width = display_width(prefix);
    if prefix_width >= width {
        return fit_to_width(prefix, width);
    }

    let name_width = width - prefix_width;
    if display_width(name) <= name_width {
        return format!("{}{}", prefix, fit_to_width(name, name_width));
    }
    if name_width == 0 {
        return fit_to_width(prefix, width);
    }

    let visible_name = truncate_to_width(name, name_width - 1);
    fit_to_width(&format!("{prefix}{visible_name}…"), width)
}

fn truncate_to_width(value: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for character in value.chars() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > width {
            break;
        }
        out.push(character);
        used += character_width;
    }
    out
}

/// Truncate to `width` terminal cells and pad with spaces so row-wide styles
/// (e.g. selected) span the whole row while preserving right-hand suffixes.
fn fit_to_width(s: &str, width: usize) -> String {
    let mut out = truncate_to_width(s, width);
    let used = display_width(&out);
    if used < width {
        out.push_str(&" ".repeat(width - used));
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
        assert_eq!(format_row(' ', 3, 1, "work", None, 10), " 3 work   ");
    }

    #[test]
    fn row_format_aligns_double_digit_indices() {
        assert_eq!(format_row(ARROW_DOWN, 10, 2, "x", None, 8), "▼10 x   ");
        assert_eq!(format_row(ARROW_UP, 9, 2, "x", None, 8), "▲ 9 x   ");
    }

    #[test]
    fn row_format_ellipsizes_long_ascii_name() {
        assert_eq!(
            format_row(' ', 1, 1, "very-long-name", None, 10),
            " 1 very-l…"
        );
    }

    #[test]
    fn row_format_ellipsizes_wide_name_by_terminal_cells() {
        let row = format_row(' ', 1, 1, "界界界界", None, 9);
        assert_eq!(row, " 1 界界… ");
        assert_eq!(display_width(&row), 9);
    }

    #[test]
    fn row_format_omits_name_when_prefix_fills_width() {
        assert_eq!(format_row(' ', 1, 1, "overflow", None, 3), " 1 ");
        assert_eq!(format_row(' ', 1, 1, "overflow", None, 2), " 1");
        assert_eq!(format_row(' ', 1, 1, "overflow", Some(""), 5), " 1  ");
    }

    #[test]
    fn parses_valid_status_payload() {
        assert_eq!(
            parse_agent_status(
                r#"{"version":1,"pane_id":"terminal_7","session_id":"session-a","state":"working","updated_at_ms":42}"#,
            ),
            Some(AgentStatusUpdate {
                pane_id: 7,
                record: AgentRecord {
                    session_id: "session-a".to_owned(),
                    state: AgentState::Working,
                    updated_at_ms: 42,
                },
            })
        );
    }

    #[test]
    fn rejects_malformed_or_unsupported_status_payloads() {
        for payload in [
            "not json",
            r#"{"version":2,"pane_id":"terminal_7","session_id":"s","state":"working","updated_at_ms":1}"#,
            r#"{"version":1,"pane_id":"plugin_7","session_id":"s","state":"working","updated_at_ms":1}"#,
            r#"{"version":1,"pane_id":"terminal_7","session_id":"","state":"working","updated_at_ms":1}"#,
            r#"{"version":1,"pane_id":"terminal_7","session_id":"s","state":"unknown","updated_at_ms":1}"#,
        ] {
            assert_eq!(parse_agent_status(payload), None, "payload: {payload}");
        }
    }

    #[test]
    fn status_update_replaces_session_and_rejects_stale_message() {
        let mut records = HashMap::new();
        let update = |session: &str, state, updated_at_ms| AgentStatusUpdate {
            pane_id: 3,
            record: AgentRecord {
                session_id: session.to_owned(),
                state,
                updated_at_ms,
            },
        };

        assert!(apply_agent_status(
            &mut records,
            update("old", AgentState::Working, 10)
        ));
        assert!(!apply_agent_status(
            &mut records,
            update("old", AgentState::Waiting, 9)
        ));
        assert!(apply_agent_status(
            &mut records,
            update("new", AgentState::Idle, 11)
        ));
        assert_eq!(records.len(), 1);
        assert_eq!(records[&3].session_id, "new");
        assert_eq!(records[&3].state, AgentState::Idle);
    }

    #[test]
    fn clear_removes_current_record_but_not_when_stale() {
        let mut records = HashMap::from([(
            4,
            AgentRecord {
                session_id: "session".to_owned(),
                state: AgentState::Done,
                updated_at_ms: 20,
            },
        )]);
        let clear = |updated_at_ms| AgentStatusUpdate {
            pane_id: 4,
            record: AgentRecord {
                session_id: "session".to_owned(),
                state: AgentState::Clear,
                updated_at_ms,
            },
        };

        assert!(!apply_agent_status(&mut records, clear(19)));
        assert!(records.contains_key(&4));
        assert!(apply_agent_status(&mut records, clear(20)));
        assert_eq!(records[&4].state, AgentState::Clear);
        assert!(!apply_agent_status(
            &mut records,
            AgentStatusUpdate {
                pane_id: 4,
                record: AgentRecord {
                    session_id: "session".to_owned(),
                    state: AgentState::Working,
                    updated_at_ms: 19,
                },
            }
        ));
        assert_eq!(records[&4].state, AgentState::Clear);
    }

    #[test]
    fn clear_from_old_session_does_not_remove_reused_pane() {
        let mut records = HashMap::from([(
            4,
            AgentRecord {
                session_id: "new-session".to_owned(),
                state: AgentState::Working,
                updated_at_ms: 20,
            },
        )]);
        let old_clear = AgentStatusUpdate {
            pane_id: 4,
            record: AgentRecord {
                session_id: "old-session".to_owned(),
                state: AgentState::Clear,
                updated_at_ms: 21,
            },
        };

        assert!(!apply_agent_status(&mut records, old_clear));
        assert_eq!(records[&4].session_id, "new-session");
        assert_eq!(records[&4].state, AgentState::Working);
    }

    #[test]
    fn pane_manifest_maps_only_terminal_panes() {
        let manifest = PaneManifest {
            panes: HashMap::from([(
                2,
                vec![
                    PaneInfo {
                        id: 8,
                        is_plugin: false,
                        ..Default::default()
                    },
                    PaneInfo {
                        id: 9,
                        is_plugin: true,
                        ..Default::default()
                    },
                ],
            )]),
        };
        assert_eq!(terminal_pane_tabs(&manifest), HashMap::from([(8, 2)]));
    }

    #[test]
    fn clear_tombstones_are_not_aggregated() {
        let records = HashMap::from([(
            4,
            AgentRecord {
                session_id: "session".to_owned(),
                state: AgentState::Clear,
                updated_at_ms: 20,
            },
        )]);
        let pane_tabs = HashMap::from([(4, 0)]);

        assert!(aggregate_agent_statuses(&records, &pane_tabs).is_empty());
    }

    #[test]
    fn discovers_only_sibling_sidebar_plugin_instances() {
        let sidebar = "file:/plugins/zellij_vertical_tab.wasm";
        let manifest = PaneManifest {
            panes: HashMap::from([
                (
                    0,
                    vec![PaneInfo {
                        id: 2,
                        is_plugin: true,
                        plugin_url: Some(sidebar.to_owned()),
                        ..Default::default()
                    }],
                ),
                (
                    1,
                    vec![
                        PaneInfo {
                            id: 5,
                            is_plugin: true,
                            plugin_url: Some(sidebar.to_owned()),
                            ..Default::default()
                        },
                        PaneInfo {
                            id: 6,
                            is_plugin: true,
                            plugin_url: Some("zellij:status-bar".to_owned()),
                            ..Default::default()
                        },
                    ],
                ),
            ]),
        };

        assert_eq!(sidebar_plugin_peers(&manifest, 2), HashSet::from([5]));
        assert_eq!(sidebar_plugin_peers(&manifest, 5), HashSet::from([2]));
        assert!(sidebar_plugin_peers(&manifest, 99).is_empty());
    }

    #[test]
    fn pane_cleanup_removes_closed_pane_records() {
        let record = |state| AgentRecord {
            session_id: "session".to_owned(),
            state,
            updated_at_ms: 1,
        };
        let mut records = HashMap::from([
            (1, record(AgentState::Working)),
            (2, record(AgentState::Done)),
        ]);
        let pane_tabs = HashMap::from([(2, 0)]);

        assert!(remove_missing_agent_records(&mut records, &pane_tabs));
        assert_eq!(records.keys().copied().collect::<Vec<_>>(), vec![2]);
        assert!(!remove_missing_agent_records(&mut records, &pane_tabs));
    }

    #[test]
    fn aggregation_counts_panes_and_uses_state_precedence() {
        let record = |state| AgentRecord {
            session_id: "session".to_owned(),
            state,
            updated_at_ms: 1,
        };
        let records = HashMap::from([
            (1, record(AgentState::Idle)),
            (2, record(AgentState::Working)),
            (3, record(AgentState::Waiting)),
            (4, record(AgentState::Done)),
        ]);
        let pane_tabs = HashMap::from([(1, 0), (2, 0), (3, 0), (4, 1)]);

        let summaries = aggregate_agent_statuses(&records, &pane_tabs);
        assert_eq!(
            summaries[&0],
            TabAgentSummary {
                state: AgentState::Waiting,
                count: 3,
            }
        );
        assert_eq!(summaries[&0].badge(), "3");
        assert_eq!(summaries[&1].badge(), "");
    }

    #[test]
    fn snapshot_round_trip_merges_newer_records_only() {
        let source = HashMap::from([
            (
                1,
                AgentRecord {
                    session_id: "one".to_owned(),
                    state: AgentState::Idle,
                    updated_at_ms: 10,
                },
            ),
            (
                2,
                AgentRecord {
                    session_id: "two".to_owned(),
                    state: AgentState::Working,
                    updated_at_ms: 20,
                },
            ),
        ]);
        let payload = serialize_agent_snapshot(&source).unwrap();
        let mut destination = HashMap::from([(
            1,
            AgentRecord {
                session_id: "newer".to_owned(),
                state: AgentState::Waiting,
                updated_at_ms: 11,
            },
        )]);

        assert!(apply_agent_snapshot(&mut destination, &payload));
        assert_eq!(destination[&1].session_id, "newer");
        assert_eq!(destination[&1].state, AgentState::Waiting);
        assert_eq!(destination[&2], source[&2]);
        assert!(!apply_agent_snapshot(&mut destination, &payload));
        assert!(!apply_agent_snapshot(
            &mut destination,
            r#"{"version":2,"records":[]}"#
        ));
    }

    #[test]
    fn badge_is_right_aligned_and_preserved_when_name_is_long() {
        assert_eq!(format_row(' ', 1, 1, "work", Some(""), 10), " 1 work  ");
        assert_eq!(
            format_row(' ', 1, 1, "very-long-name", Some("2"), 10),
            " 1 ver… 2"
        );
        assert_eq!(format_row(' ', 1, 1, "x", Some("2"), 2), "2");
        assert_eq!(
            format_row(' ', 1, 1, "界界界界", Some(""), 10),
            " 1 界界… "
        );
        assert_eq!(display_width(" 1 界界… "), 10);
    }

    #[test]
    fn status_glyphs_are_single_cell_nerd_font_icons() {
        for state in [
            AgentState::Idle,
            AgentState::Working,
            AgentState::Waiting,
            AgentState::Done,
        ] {
            assert_eq!(display_width(state.glyph()), 1);
        }
    }

    #[test]
    fn status_states_use_distinct_theme_colors() {
        assert_eq!(AgentState::Idle.badge_color(), BadgeColor::Dim);
        assert_eq!(AgentState::Working.badge_color(), BadgeColor::Emphasis(1));
        assert_eq!(AgentState::Waiting.badge_color(), BadgeColor::Emphasis(0));
        assert_eq!(AgentState::Done.badge_color(), BadgeColor::Success);
        assert_eq!(AgentState::Clear.badge_color(), BadgeColor::None);
    }

    #[test]
    fn complete_badge_is_colored_and_selected_row_is_retained() {
        let text = color_agent_badge(Text::new(" 1 work  2"), AgentState::Working, 2).selected();
        let serialized = text.serialize();

        assert!(serialized.starts_with("x$9,10$"));
    }
}
