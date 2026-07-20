//! zellij-vertical-tab: renders the session's tabs vertically, with pane
//! children for multi-pane tabs, inside a flexible unselectable side pane.
//!
//! Interactions:
//! - left-click a tab row to switch tabs or a pane row to focus that pane
//! - scroll wheel moves the visible window when tabs overflow the pane height
//! - the active tab is always kept inside the visible window
//!
//! Tab rows use `<lead><name>`; native nested-list pane rows appear
//! immediately
//! below tabs with multiple terminals. `lead` is '▲' on the first visible row
//! when hierarchy rows are hidden above, '▼' on the last visible row when rows
//! are hidden below, and ' ' otherwise.
//! Styling comes from Zellij's native nested-list component, which renders
//! hierarchy bullets plus selected and unselected rows with the user's theme.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;
use zellij_tile::prelude::*;

const ARROW_UP: char = '▲';
const ARROW_DOWN: char = '▼';
const AGENT_STATUS_PIPE: &str = "vertical-tab-agent-status";
const AGENT_STATUS_SYNC_UPDATE: &str = "vertical-tab-agent-status-sync-update";
const AGENT_STATUS_SYNC_REQUEST: &str = "vertical-tab-agent-status-sync-request";
const AGENT_STATUS_SYNC_SNAPSHOT: &str = "vertical-tab-agent-status-sync-snapshot";
const AGENT_STATUS_SYNC_ACKNOWLEDGEMENT: &str = "vertical-tab-agent-status-sync-acknowledgement";
const AGENT_STATUS_SYNC_FOCUS: &str = "vertical-tab-agent-status-sync-focus";
const AGENT_STATUS_RESTORE_CONTEXT: &str = "vertical-tab-agent-status-restore-v1";
const AGENT_STATUS_VERSION: u8 = 1;
const AGENT_CACHE_PREFIX: &str = "agent-status";
const AGENT_CACHE_MAX_BYTES: usize = 1024 * 1024;
const HOST_RESTORE_MAX_BYTES: usize = 1024 * 1024;
const HOST_RESTORE_SCRIPT: &str =
    r#"exec "${CODEX_HOME:-$HOME/.codex}/hooks/agent_status.py" --snapshot "$1""#;
const ROW_RIGHT_PADDING: usize = 1;
const TAB_LIST_CHROME_WIDTH: usize = 3;
const PANE_LIST_CHROME_WIDTH: usize = 5;
const ATTENTION_GLYPH: &str = "";

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
    #[serde(default)]
    acknowledgements: Vec<AgentAcknowledgementPayload>,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct AgentAcknowledgementPayload {
    version: u8,
    pane_id: String,
    session_id: String,
    updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AgentAcknowledgement {
    session_id: String,
    updated_at_ms: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct AgentAcknowledgementUpdate {
    pane_id: u32,
    acknowledgement: AgentAcknowledgement,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct AgentFocusPayload {
    version: u8,
    pane_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerminalPane {
    id: u32,
    title: String,
    is_focused: bool,
    is_floating: bool,
    is_suppressed: bool,
    pane_x: usize,
    pane_y: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RowTarget {
    Tab { position: usize },
    Pane { id: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SidebarRow {
    Tab {
        position: usize,
        name: String,
        active: bool,
        state: Option<AgentState>,
        attention: bool,
    },
    Pane {
        id: u32,
        title: String,
        focused: bool,
        state: Option<AgentState>,
    },
}

impl SidebarRow {
    fn target(&self) -> RowTarget {
        match self {
            Self::Tab { position, .. } => RowTarget::Tab {
                position: *position,
            },
            Self::Pane { id, .. } => RowTarget::Pane { id: *id },
        }
    }

    fn is_selected(&self) -> bool {
        match self {
            Self::Tab { active, .. } => *active,
            Self::Pane { focused, .. } => *focused,
        }
    }

    fn state(&self) -> Option<AgentState> {
        match self {
            Self::Tab { state, .. } | Self::Pane { state, .. } => *state,
        }
    }

    fn has_attention(&self) -> bool {
        matches!(
            self,
            Self::Tab {
                attention: true,
                ..
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
struct NativeListRow {
    content: String,
    indentation: usize,
    selected: bool,
    state: Option<AgentState>,
    badge_chars: usize,
    attention_chars: usize,
    trailing_chars: usize,
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
    /// Display metadata for terminal panes, grouped and ordered by tab.
    terminal_panes: HashMap<usize, Vec<TerminalPane>>,
    /// Most recent top-level Codex session status for each terminal pane.
    agent_records: HashMap<u32, AgentRecord>,
    /// Exact completed lifecycle records acknowledged through terminal focus.
    agent_acknowledgements: HashMap<u32, AgentAcknowledgement>,
    /// Terminal panes viewed by attached clients at the last complete focus observation.
    focused_terminal_panes: Option<HashSet<u32>>,
    /// This sidebar instance's session-unique Zellij plugin ID.
    plugin_id: Option<u32>,
    /// Zellij server PID used to isolate durable state between live sessions.
    zellij_pid: Option<u32>,
    /// Other sidebar plugin instances discovered across the session's tabs.
    peer_plugin_ids: HashSet<u32>,
    /// Whether `set_selectable(false)` has been applied yet.
    unselectable_set: bool,
    /// Whether this runtime already requested one host-journal snapshot.
    host_restore_requested: bool,
}

register_plugin!(State);

impl State {
    fn persist_agent_cache(&self) {
        if let (Some(zellij_pid), Some(plugin_id)) = (self.zellij_pid, self.plugin_id) {
            persist_agent_cache(
                Path::new("/cache"),
                zellij_pid,
                plugin_id,
                &self.agent_records,
                &self.agent_acknowledgements,
            );
        }
    }

    fn apply_snapshot_payload(&mut self, payload: &str) -> bool {
        let changed = apply_agent_snapshot(
            &mut self.agent_records,
            &mut self.agent_acknowledgements,
            payload,
        );
        if changed {
            self.persist_agent_cache();
        }
        changed
    }

    fn request_host_restore(&mut self) {
        if self.host_restore_requested {
            return;
        }
        let Some(zellij_pid) = self.zellij_pid else {
            return;
        };
        self.host_restore_requested = true;
        let zellij_pid = zellij_pid.to_string();
        let context = BTreeMap::from([(
            "request".to_owned(),
            AGENT_STATUS_RESTORE_CONTEXT.to_owned(),
        )]);
        run_command(
            &[
                "/bin/sh",
                "-c",
                HOST_RESTORE_SCRIPT,
                "zellij-vertical-tab",
                &zellij_pid,
            ],
            context,
        );
    }

    fn sidebar_rows(&self) -> Vec<SidebarRow> {
        build_sidebar_rows(
            &self.tabs,
            &self.terminal_panes,
            &self.agent_records,
            &self.agent_acknowledgements,
        )
    }

    fn accept_focus_observation(&mut self, focused_panes: HashSet<u32>) -> bool {
        let Some(previous_focused_panes) =
            self.focused_terminal_panes.replace(focused_panes.clone())
        else {
            return false;
        };
        let updates = newly_focused_done_acknowledgements(
            &previous_focused_panes,
            &focused_panes,
            &self.agent_records,
        );
        let mut changed = false;
        for update in updates {
            let pane_id = update.pane_id;
            if apply_agent_acknowledgement(&mut self.agent_acknowledgements, update) {
                changed = true;
                if let Some(payload) =
                    serialize_agent_acknowledgement(pane_id, &self.agent_acknowledgements[&pane_id])
                {
                    for peer_id in &self.peer_plugin_ids {
                        send_plugin_message(
                            *peer_id,
                            AGENT_STATUS_SYNC_ACKNOWLEDGEMENT,
                            payload.clone(),
                        );
                    }
                }
            }
        }
        if changed {
            self.persist_agent_cache();
        }
        changed
    }

    fn reconcile_focus_transition(&mut self) -> bool {
        // TabUpdate and PaneUpdate can initialize in either order. Establish a
        // baseline only after both halves of the focus view are available so
        // plugin startup or hot reload is not mistaken for a user transition.
        if self.tabs.is_empty() || self.terminal_panes.is_empty() {
            return false;
        }

        let focused_panes: HashSet<u32> =
            focused_terminal_pane_ids(&self.tabs, &self.terminal_panes)
                .into_iter()
                .collect();
        if self.focused_terminal_panes.as_ref() == Some(&focused_panes) {
            return false;
        }
        let payload = serialize_agent_focus(&focused_panes);
        let acknowledgement_changed = self.accept_focus_observation(focused_panes);
        if let Some(payload) = payload {
            for peer_id in &self.peer_plugin_ids {
                send_plugin_message(*peer_id, AGENT_STATUS_SYNC_FOCUS, payload.clone());
            }
        }
        acknowledgement_changed
    }

    fn accept_agent_status(&mut self, update: AgentStatusUpdate) -> bool {
        let pane_id = update.pane_id;
        let record = update.record.clone();
        let status_changed = apply_agent_status(&mut self.agent_records, update);
        let acknowledgement_pruned = status_changed
            && prune_superseded_agent_acknowledgement(
                &mut self.agent_acknowledgements,
                pane_id,
                &record,
            );
        let changed = status_changed || acknowledgement_pruned;
        if changed {
            self.persist_agent_cache();
        }
        changed
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        let plugin_ids = get_plugin_ids();
        self.plugin_id = Some(plugin_ids.plugin_id);
        self.zellij_pid = Some(plugin_ids.zellij_pid);
        restore_agent_cache(
            Path::new("/cache"),
            plugin_ids.zellij_pid,
            &mut self.agent_records,
            &mut self.agent_acknowledgements,
        );
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::ReadCliPipes,
            PermissionType::MessageAndLaunchOtherPlugins,
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::Mouse,
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
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
                let sidebar_rows = self.sidebar_rows();
                let active_row = active_tab_row(&sidebar_rows);
                if new_active != self.active_idx {
                    // The user switched tabs outside the plugin (keybindings,
                    // etc.): follow the active tab so it stays visible.
                    self.active_idx = new_active;
                    self.scroll_offset = visible_window(
                        sidebar_rows.len(),
                        active_row,
                        self.scroll_offset,
                        self.rows,
                    );
                } else {
                    self.scroll_offset =
                        clamp_offset(sidebar_rows.len(), self.scroll_offset, self.rows);
                }
                self.reconcile_focus_transition();
                true
            }
            Event::PaneUpdate(pane_manifest) => {
                let pane_tabs = terminal_pane_tabs(&pane_manifest);
                let terminal_panes = terminal_panes_by_tab(&pane_manifest);
                let records_removed =
                    remove_missing_agent_records(&mut self.agent_records, &pane_tabs);
                let acknowledgements_removed = remove_missing_agent_acknowledgements(
                    &mut self.agent_acknowledgements,
                    &pane_tabs,
                );
                let panes_changed =
                    pane_tabs != self.pane_tabs || terminal_panes != self.terminal_panes;
                self.pane_tabs = pane_tabs;
                self.terminal_panes = terminal_panes;
                let sidebar_rows = self.sidebar_rows();
                self.scroll_offset = visible_window(
                    sidebar_rows.len(),
                    active_tab_row(&sidebar_rows),
                    self.scroll_offset,
                    self.rows,
                );
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
                    if let Some(focused_panes) = self.focused_terminal_panes.as_ref() {
                        if let Some(payload) = serialize_agent_focus(focused_panes) {
                            send_plugin_message(*peer_id, AGENT_STATUS_SYNC_FOCUS, payload);
                        }
                    }
                }
                let peers_changed = peers != self.peer_plugin_ids;
                self.peer_plugin_ids = peers;
                let acknowledgement_changed = self.reconcile_focus_transition();
                let changed = records_removed
                    || acknowledgements_removed
                    || panes_changed
                    || peers_changed
                    || acknowledgement_changed;
                if records_removed || acknowledgements_removed {
                    self.persist_agent_cache();
                }
                changed
            }
            Event::Mouse(mouse) => match mouse {
                Mouse::LeftClick(line, _col) => {
                    // Mouse coordinates are 0-based content cells; `line` is
                    // signed because it can go negative in scrollback (not
                    // possible here, but guard anyway).
                    if line >= 0 {
                        let idx = self.scroll_offset + line as usize;
                        let sidebar_rows = self.sidebar_rows();
                        if let Some(row) = sidebar_rows.get(idx) {
                            match row.target() {
                                RowTarget::Tab { position } => {
                                    // Tab indices are 1-based when switching.
                                    switch_tab_to(position as u32 + 1)
                                }
                                RowTarget::Pane { id } => focus_terminal_pane(id, false, false),
                            }
                        }
                    }
                    // The resulting TabUpdate or PaneUpdate triggers rendering.
                    false
                }
                Mouse::ScrollUp(_) => {
                    let new_offset = self.scroll_offset.saturating_sub(1);
                    std::mem::replace(&mut self.scroll_offset, new_offset) != new_offset
                }
                Mouse::ScrollDown(_) => {
                    let row_count = self.sidebar_rows().len();
                    let new_offset = clamp_offset(row_count, self.scroll_offset + 1, self.rows);
                    std::mem::replace(&mut self.scroll_offset, new_offset) != new_offset
                }
                _ => false,
            },
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.request_host_restore();
                false
            }
            Event::RunCommandResult(exit_code, stdout, _stderr, context) => {
                if context.get("request").map(String::as_str) != Some(AGENT_STATUS_RESTORE_CONTEXT)
                    || exit_code != Some(0)
                    || stdout.len() > HOST_RESTORE_MAX_BYTES
                {
                    return false;
                }
                std::str::from_utf8(&stdout)
                    .ok()
                    .is_some_and(|payload| self.apply_snapshot_payload(payload.trim()))
            }
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
                let changed = self.accept_agent_status(update);
                for peer_id in &self.peer_plugin_ids {
                    send_plugin_message(*peer_id, AGENT_STATUS_SYNC_UPDATE, payload.to_owned());
                }
                changed
            }
            AGENT_STATUS_SYNC_UPDATE => {
                parse_agent_status(payload).is_some_and(|update| self.accept_agent_status(update))
            }
            AGENT_STATUS_SYNC_REQUEST => {
                let Ok(requester_id) = payload.parse::<u32>() else {
                    return false;
                };
                if Some(requester_id) == self.plugin_id {
                    return false;
                }
                if let Some(snapshot) =
                    serialize_agent_snapshot(&self.agent_records, &self.agent_acknowledgements)
                {
                    send_plugin_message(requester_id, AGENT_STATUS_SYNC_SNAPSHOT, snapshot);
                }
                false
            }
            AGENT_STATUS_SYNC_SNAPSHOT => self.apply_snapshot_payload(payload),
            AGENT_STATUS_SYNC_ACKNOWLEDGEMENT => {
                let changed = parse_agent_acknowledgement(payload).is_some_and(|update| {
                    apply_agent_acknowledgement(&mut self.agent_acknowledgements, update)
                });
                if changed {
                    self.persist_agent_cache();
                }
                changed
            }
            AGENT_STATUS_SYNC_FOCUS => parse_agent_focus(payload)
                .is_some_and(|focused_panes| self.accept_focus_observation(focused_panes)),
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let sidebar_rows = self.sidebar_rows();
        if rows != self.rows {
            self.rows = rows;
            self.scroll_offset = visible_window(
                sidebar_rows.len(),
                active_tab_row(&sidebar_rows),
                self.scroll_offset,
                rows,
            );
        }
        if sidebar_rows.is_empty() || rows == 0 || cols == 0 {
            return;
        }

        let offset = self.scroll_offset;
        let visible_count = rows.min(sidebar_rows.len() - offset);
        let native_rows = (0..visible_count).map(|i| {
            let sidebar_row = &sidebar_rows[offset + i];
            let lead = if i == 0 && offset > 0 {
                ARROW_UP
            } else if i == visible_count - 1 && offset + visible_count < sidebar_rows.len() {
                ARROW_DOWN
            } else {
                ' '
            };
            native_list_row(sidebar_row, lead, cols)
        });
        let items = native_rows.map(native_list_item).collect();
        print_nested_list_with_coordinates(items, 0, 0, Some(cols), Some(visible_count));
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

fn parse_agent_acknowledgement(payload: &str) -> Option<AgentAcknowledgementUpdate> {
    let payload: AgentAcknowledgementPayload = serde_json::from_str(payload).ok()?;
    if payload.version != AGENT_STATUS_VERSION
        || payload.session_id.trim().is_empty()
        || payload.updated_at_ms == 0
    {
        return None;
    }
    Some(AgentAcknowledgementUpdate {
        pane_id: parse_terminal_pane_id(&payload.pane_id)?,
        acknowledgement: AgentAcknowledgement {
            session_id: payload.session_id,
            updated_at_ms: payload.updated_at_ms,
        },
    })
}

fn serialize_agent_acknowledgement(
    pane_id: u32,
    acknowledgement: &AgentAcknowledgement,
) -> Option<String> {
    serde_json::to_string(&AgentAcknowledgementPayload {
        version: AGENT_STATUS_VERSION,
        pane_id: format!("terminal_{pane_id}"),
        session_id: acknowledgement.session_id.clone(),
        updated_at_ms: acknowledgement.updated_at_ms,
    })
    .ok()
}

fn parse_agent_focus(payload: &str) -> Option<HashSet<u32>> {
    let payload: AgentFocusPayload = serde_json::from_str(payload).ok()?;
    if payload.version != AGENT_STATUS_VERSION {
        return None;
    }
    let pane_count = payload.pane_ids.len();
    let focused_panes: HashSet<u32> = payload
        .pane_ids
        .iter()
        .map(|pane_id| parse_terminal_pane_id(pane_id))
        .collect::<Option<_>>()?;
    (focused_panes.len() == pane_count).then_some(focused_panes)
}

fn serialize_agent_focus(focused_panes: &HashSet<u32>) -> Option<String> {
    let mut pane_ids: Vec<String> = focused_panes
        .iter()
        .map(|pane_id| format!("terminal_{pane_id}"))
        .collect();
    pane_ids.sort();
    serde_json::to_string(&AgentFocusPayload {
        version: AGENT_STATUS_VERSION,
        pane_ids,
    })
    .ok()
}

fn serialize_agent_snapshot(
    records: &HashMap<u32, AgentRecord>,
    acknowledgements: &HashMap<u32, AgentAcknowledgement>,
) -> Option<String> {
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
    let mut acknowledgements = acknowledgements
        .iter()
        .map(|(pane_id, acknowledgement)| AgentAcknowledgementPayload {
            version: AGENT_STATUS_VERSION,
            pane_id: format!("terminal_{pane_id}"),
            session_id: acknowledgement.session_id.clone(),
            updated_at_ms: acknowledgement.updated_at_ms,
        })
        .collect::<Vec<_>>();
    acknowledgements.sort_by(|left, right| left.pane_id.cmp(&right.pane_id));
    serde_json::to_string(&AgentStatusSnapshot {
        version: AGENT_STATUS_VERSION,
        records,
        acknowledgements,
    })
    .ok()
}

fn apply_agent_snapshot(
    records: &mut HashMap<u32, AgentRecord>,
    acknowledgements: &mut HashMap<u32, AgentAcknowledgement>,
    payload: &str,
) -> bool {
    let Ok(snapshot) = serde_json::from_str::<AgentStatusSnapshot>(payload) else {
        return false;
    };
    if snapshot.version != AGENT_STATUS_VERSION {
        return false;
    }
    let records_changed = snapshot.records.into_iter().fold(false, |changed, record| {
        let update = serde_json::to_string(&record)
            .ok()
            .and_then(|record| parse_agent_status(&record));
        update.map_or(changed, |update| {
            let pane_id = update.pane_id;
            let record = update.record.clone();
            let status_changed = apply_agent_status(records, update);
            let acknowledgement_pruned = status_changed
                && prune_superseded_agent_acknowledgement(acknowledgements, pane_id, &record);
            changed || status_changed || acknowledgement_pruned
        })
    });
    snapshot
        .acknowledgements
        .into_iter()
        .fold(records_changed, |changed, acknowledgement| {
            let update = serde_json::to_string(&acknowledgement)
                .ok()
                .and_then(|acknowledgement| parse_agent_acknowledgement(&acknowledgement));
            update.map_or(changed, |update| {
                apply_agent_acknowledgement(acknowledgements, update) || changed
            })
        })
}

fn agent_cache_path(cache_dir: &Path, zellij_pid: u32, plugin_id: u32) -> std::path::PathBuf {
    cache_dir.join(format!(
        "{AGENT_CACHE_PREFIX}-{zellij_pid}-{plugin_id}.json"
    ))
}

fn parse_agent_cache_filename(file_name: &str, zellij_pid: u32) -> Option<u32> {
    let stem = file_name.strip_suffix(".json")?;
    let suffix = stem.strip_prefix(&format!("{AGENT_CACHE_PREFIX}-{zellij_pid}-"))?;
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
}

fn persist_agent_cache(
    cache_dir: &Path,
    zellij_pid: u32,
    plugin_id: u32,
    records: &HashMap<u32, AgentRecord>,
    acknowledgements: &HashMap<u32, AgentAcknowledgement>,
) -> bool {
    let Some(snapshot) = serialize_agent_snapshot(records, acknowledgements) else {
        return false;
    };
    if snapshot.len() > AGENT_CACHE_MAX_BYTES || std::fs::create_dir_all(cache_dir).is_err() {
        return false;
    }
    let destination = agent_cache_path(cache_dir, zellij_pid, plugin_id);
    let temporary = cache_dir.join(format!(
        ".{AGENT_CACHE_PREFIX}-{zellij_pid}-{plugin_id}.tmp"
    ));
    if std::fs::write(&temporary, snapshot).is_err() {
        return false;
    }
    if std::fs::rename(&temporary, destination).is_err() {
        let _ = std::fs::remove_file(temporary);
        return false;
    }
    true
}

fn restore_agent_cache(
    cache_dir: &Path,
    zellij_pid: u32,
    records: &mut HashMap<u32, AgentRecord>,
    acknowledgements: &mut HashMap<u32, AgentAcknowledgement>,
) -> bool {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return false;
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_str()?;
            parse_agent_cache_filename(file_name, zellij_pid)?;
            Some(entry.path())
        })
        .collect::<Vec<_>>();
    paths.sort();

    paths.into_iter().fold(false, |changed, path| {
        let payload = std::fs::metadata(&path)
            .ok()
            .filter(|metadata| metadata.is_file() && metadata.len() <= AGENT_CACHE_MAX_BYTES as u64)
            .and_then(|_| std::fs::read_to_string(path).ok());
        payload.map_or(changed, |payload| {
            apply_agent_snapshot(records, acknowledgements, &payload) || changed
        })
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

fn apply_agent_acknowledgement(
    acknowledgements: &mut HashMap<u32, AgentAcknowledgement>,
    update: AgentAcknowledgementUpdate,
) -> bool {
    if acknowledgements
        .get(&update.pane_id)
        .is_some_and(|current| {
            update.acknowledgement.updated_at_ms < current.updated_at_ms
                || *current == update.acknowledgement
        })
    {
        return false;
    }
    acknowledgements.insert(update.pane_id, update.acknowledgement);
    true
}

fn prune_superseded_agent_acknowledgement(
    acknowledgements: &mut HashMap<u32, AgentAcknowledgement>,
    pane_id: u32,
    record: &AgentRecord,
) -> bool {
    let should_remove = acknowledgements
        .get(&pane_id)
        .is_some_and(|acknowledgement| {
            (acknowledgement.session_id != record.session_id
                || acknowledgement.updated_at_ms != record.updated_at_ms)
                && acknowledgement.updated_at_ms <= record.updated_at_ms
        });
    should_remove && acknowledgements.remove(&pane_id).is_some()
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

fn terminal_panes_by_tab(pane_manifest: &PaneManifest) -> HashMap<usize, Vec<TerminalPane>> {
    pane_manifest
        .panes
        .iter()
        .map(|(tab_position, panes)| {
            let mut terminal_panes = panes
                .iter()
                .filter(|pane| !pane.is_plugin)
                .map(|pane| TerminalPane {
                    id: pane.id,
                    title: pane.title.clone(),
                    is_focused: pane.is_focused,
                    is_floating: pane.is_floating,
                    is_suppressed: pane.is_suppressed,
                    pane_x: pane.pane_x,
                    pane_y: pane.pane_y,
                })
                .collect::<Vec<_>>();
            terminal_panes.sort_by_key(pane_sort_key);
            (*tab_position, terminal_panes)
        })
        .collect()
}

fn pane_sort_key(pane: &TerminalPane) -> (u8, usize, usize, u32) {
    let layer = if pane.is_suppressed {
        2
    } else if pane.is_floating {
        1
    } else {
        0
    };
    (layer, pane.pane_y, pane.pane_x, pane.id)
}

fn renderable_agent_state(
    records: &HashMap<u32, AgentRecord>,
    acknowledgements: &HashMap<u32, AgentAcknowledgement>,
    pane_id: u32,
) -> Option<AgentState> {
    let record = records.get(&pane_id)?;
    if record.state == AgentState::Clear {
        return None;
    }
    let acknowledged = record.state == AgentState::Done
        && acknowledgements
            .get(&pane_id)
            .is_some_and(|acknowledgement| {
                acknowledgement.session_id == record.session_id
                    && acknowledgement.updated_at_ms == record.updated_at_ms
            });
    Some(if acknowledged {
        AgentState::Idle
    } else {
        record.state
    })
}

fn build_sidebar_rows(
    tabs: &[TabInfo],
    terminal_panes: &HashMap<usize, Vec<TerminalPane>>,
    records: &HashMap<u32, AgentRecord>,
    acknowledgements: &HashMap<u32, AgentAcknowledgement>,
) -> Vec<SidebarRow> {
    let mut rows = Vec::new();
    for tab in tabs {
        let panes = terminal_panes
            .get(&tab.position)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let state = if panes.len() == 1 {
            renderable_agent_state(records, acknowledgements, panes[0].id)
        } else {
            None
        };
        rows.push(SidebarRow::Tab {
            position: tab.position,
            name: tab.name.clone(),
            active: tab.active,
            state,
            attention: tab.has_bell_notification,
        });

        if panes.len() > 1 {
            let focused_pane_id = focused_pane_id(tab, panes);
            rows.extend(panes.iter().map(|pane| SidebarRow::Pane {
                id: pane.id,
                title: if pane.title.is_empty() {
                    format!("pane {}", pane.id)
                } else {
                    pane.title.clone()
                },
                focused: focused_pane_id == Some(pane.id),
                state: renderable_agent_state(records, acknowledgements, pane.id),
            }));
        }
    }
    rows
}

fn focused_terminal_pane_ids(
    tabs: &[TabInfo],
    terminal_panes: &HashMap<usize, Vec<TerminalPane>>,
) -> Vec<u32> {
    let has_attached_client_focus = tabs.iter().any(|tab| !tab.other_focused_clients.is_empty());
    tabs.iter()
        .filter(|tab| {
            if has_attached_client_focus {
                !tab.other_focused_clients.is_empty()
            } else {
                tab.active
            }
        })
        .filter_map(|tab| {
            focused_pane_in_tab(
                tab,
                terminal_panes
                    .get(&tab.position)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )
        })
        .collect()
}

fn newly_focused_done_acknowledgements(
    previous_focused_panes: &HashSet<u32>,
    focused_panes: &HashSet<u32>,
    records: &HashMap<u32, AgentRecord>,
) -> Vec<AgentAcknowledgementUpdate> {
    let mut newly_focused_panes: Vec<u32> = focused_panes
        .difference(previous_focused_panes)
        .copied()
        .collect();
    newly_focused_panes.sort_unstable();
    newly_focused_panes
        .into_iter()
        .filter_map(|pane_id| {
            let record = records
                .get(&pane_id)
                .filter(|record| record.state == AgentState::Done)?;
            Some(AgentAcknowledgementUpdate {
                pane_id,
                acknowledgement: AgentAcknowledgement {
                    session_id: record.session_id.clone(),
                    updated_at_ms: record.updated_at_ms,
                },
            })
        })
        .collect()
}

fn focused_pane_id(tab: &TabInfo, panes: &[TerminalPane]) -> Option<u32> {
    if !tab.active {
        return None;
    }
    focused_pane_in_tab(tab, panes)
}

fn focused_pane_in_tab(tab: &TabInfo, panes: &[TerminalPane]) -> Option<u32> {
    panes
        .iter()
        .find(|pane| {
            pane.is_focused
                && !pane.is_suppressed
                && pane.is_floating == tab.are_floating_panes_visible
        })
        .or_else(|| {
            panes
                .iter()
                .find(|pane| pane.is_focused && !pane.is_suppressed)
        })
        .map(|pane| pane.id)
}

fn active_tab_row(rows: &[SidebarRow]) -> Option<usize> {
    rows.iter()
        .position(|row| matches!(row, SidebarRow::Tab { active: true, .. }))
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

fn remove_missing_agent_acknowledgements(
    acknowledgements: &mut HashMap<u32, AgentAcknowledgement>,
    pane_tabs: &HashMap<u32, usize>,
) -> bool {
    let old_count = acknowledgements.len();
    acknowledgements.retain(|pane_id, _| pane_tabs.contains_key(pane_id));
    old_count != acknowledgements.len()
}

/// Largest valid scroll offset: 0 when everything fits.
fn clamp_offset(row_count: usize, offset: usize, rows: usize) -> usize {
    if rows == 0 || row_count <= rows {
        0
    } else {
        offset.min(row_count - rows)
    }
}

/// Scroll offset that keeps the active tab row inside the `rows`-high window
/// while moving the previous offset as little as possible.
fn visible_window(row_count: usize, active: Option<usize>, prev: usize, rows: usize) -> usize {
    let mut offset = clamp_offset(row_count, prev, rows);
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
fn format_row(lead: char, name: &str, badge: Option<&str>, width: usize) -> String {
    let prefix = lead.to_string();
    format_named_row(&prefix, name, badge, false, width)
}

/// Build an indented pane-child row with an optional right-aligned badge.
fn format_pane_row(lead: char, title: &str, badge: Option<&str>, width: usize) -> String {
    let prefix = lead.to_string();
    format_named_row(&prefix, title, badge, false, width)
}

fn native_list_row(sidebar_row: &SidebarRow, lead: char, cols: usize) -> NativeListRow {
    let state = sidebar_row.state();
    let attention = sidebar_row.has_attention();
    let (indentation, content_width) = match sidebar_row {
        SidebarRow::Tab { .. } => {
            let content_width = cols.saturating_sub(TAB_LIST_CHROME_WIDTH);
            (0, content_width)
        }
        SidebarRow::Pane { .. } => {
            let content_width = cols.saturating_sub(PANE_LIST_CHROME_WIDTH);
            (1, content_width)
        }
    };
    let requested_badge = state.map(AgentState::glyph);
    let badge = if attention
        && format_suffix(requested_badge, true)
            .is_some_and(|suffix| display_width(&suffix) > content_width)
    {
        None
    } else {
        requested_badge
    };
    let content = match sidebar_row {
        SidebarRow::Tab { name, .. } if attention => {
            format_named_row(&lead.to_string(), name, badge, true, content_width)
        }
        SidebarRow::Tab { name, .. } => format_row(lead, name, badge, content_width),
        SidebarRow::Pane { title, .. } => format_pane_row(lead, title, badge, content_width),
    };
    let suffix = format_suffix(badge, attention);
    let trailing_chars = suffix
        .as_deref()
        .map(|suffix| badge_right_padding(content_width, display_width(suffix)))
        .unwrap_or_else(|| row_right_padding(content_width, display_width(&lead.to_string())));
    NativeListRow {
        content,
        indentation,
        selected: sidebar_row.is_selected(),
        state: badge.and(state),
        badge_chars: badge.map(|badge| badge.chars().count()).unwrap_or_default(),
        attention_chars: usize::from(attention) * ATTENTION_GLYPH.chars().count(),
        trailing_chars,
    }
}

fn native_list_item(row: NativeListRow) -> NestedListItem {
    let content_chars = row.content.chars().count();
    let mut item = NestedListItem::new(row.content).indent(row.indentation);
    if let Some(state) = row.state {
        item = color_agent_badge_item(
            item,
            state,
            content_chars,
            row.badge_chars,
            row.attention_chars,
            row.trailing_chars,
        );
    }
    if row.attention_chars > 0 {
        item = color_attention_badge_item(
            item,
            content_chars,
            row.attention_chars,
            row.trailing_chars,
        );
    }
    if row.selected {
        item = item.selected();
    }
    item
}

fn format_suffix(badge: Option<&str>, attention: bool) -> Option<String> {
    match (badge.filter(|badge| !badge.is_empty()), attention) {
        (Some(badge), true) => Some(format!("{badge} {ATTENTION_GLYPH}")),
        (Some(badge), false) => Some(badge.to_owned()),
        (None, true) => Some(ATTENTION_GLYPH.to_owned()),
        (None, false) => None,
    }
}

fn format_named_row(
    prefix: &str,
    name: &str,
    badge: Option<&str>,
    attention: bool,
    width: usize,
) -> String {
    let Some(suffix) = format_suffix(badge, attention) else {
        let right_padding = row_right_padding(width, display_width(prefix));
        return format!(
            "{}{}",
            fit_tab_body(prefix, name, width.saturating_sub(right_padding)),
            " ".repeat(right_padding)
        );
    };
    let suffix_width = display_width(&suffix);
    let right_padding = badge_right_padding(width, suffix_width);
    let reserved_width = suffix_width + right_padding;
    if reserved_width >= width {
        return format!(
            "{}{}",
            fit_to_width(&suffix, width.saturating_sub(right_padding)),
            " ".repeat(right_padding)
        );
    }
    let body_width = width - reserved_width - 1;
    format!(
        "{} {}{}",
        fit_tab_body(prefix, name, body_width),
        suffix,
        " ".repeat(right_padding)
    )
}

fn badge_right_padding(width: usize, badge_width: usize) -> usize {
    row_right_padding(width, badge_width)
}

fn row_right_padding(width: usize, protected_width: usize) -> usize {
    ROW_RIGHT_PADDING.min(width.saturating_sub(protected_width))
}

fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|character| character.width().unwrap_or(0))
        .sum()
}

fn color_agent_badge_item(
    item: NestedListItem,
    state: AgentState,
    content_chars: usize,
    badge_chars: usize,
    attention_chars: usize,
    trailing_chars: usize,
) -> NestedListItem {
    let attention_offset = attention_chars + usize::from(attention_chars > 0);
    let badge_end = content_chars
        .saturating_sub(trailing_chars)
        .saturating_sub(attention_offset);
    let badge_start = badge_end.saturating_sub(badge_chars);
    match state.badge_color() {
        BadgeColor::Dim => item.color_range(4, badge_start..badge_end),
        BadgeColor::Emphasis(level) => item.color_range(level, badge_start..badge_end),
        BadgeColor::Success => item.success_color_range(badge_start..badge_end),
        BadgeColor::None => item,
    }
}

fn color_attention_badge_item(
    item: NestedListItem,
    content_chars: usize,
    attention_chars: usize,
    trailing_chars: usize,
) -> NestedListItem {
    let attention_end = content_chars.saturating_sub(trailing_chars);
    let attention_start = attention_end.saturating_sub(attention_chars);
    item.color_range(0, attention_start..attention_end)
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_DIRECTORY_ID: AtomicUsize = AtomicUsize::new(0);

    fn temp_directory(label: &str) -> std::path::PathBuf {
        let id = TEMP_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zellij-vertical-tab-{label}-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn tab(position: usize, name: &str, active: bool) -> TabInfo {
        TabInfo {
            position,
            name: name.to_owned(),
            active,
            ..Default::default()
        }
    }

    fn terminal_pane(
        id: u32,
        title: &str,
        is_focused: bool,
        is_floating: bool,
        is_suppressed: bool,
        pane_x: usize,
        pane_y: usize,
    ) -> TerminalPane {
        TerminalPane {
            id,
            title: title.to_owned(),
            is_focused,
            is_floating,
            is_suppressed,
            pane_x,
            pane_y,
        }
    }

    fn agent_record(state: AgentState) -> AgentRecord {
        AgentRecord {
            session_id: "session".to_owned(),
            state,
            updated_at_ms: 1,
        }
    }

    fn acknowledgement(session_id: &str, updated_at_ms: u64) -> AgentAcknowledgement {
        AgentAcknowledgement {
            session_id: session_id.to_owned(),
            updated_at_ms,
        }
    }

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
        assert_eq!(format_row(' ', "work", None, 10), " work     ");
    }

    #[test]
    fn row_format_keeps_overflow_lead_without_a_tab_index() {
        assert_eq!(format_row(ARROW_DOWN, "x", None, 8), "▼x      ");
        assert_eq!(format_row(ARROW_UP, "x", None, 8), "▲x      ");
    }

    #[test]
    fn row_format_ellipsizes_long_ascii_name() {
        assert_eq!(format_row(' ', "very-long-name", None, 10), " very-lo… ");
    }

    #[test]
    fn row_format_ellipsizes_wide_name_by_terminal_cells() {
        let row = format_row(' ', "界界界界", None, 9);
        assert_eq!(row, " 界界界… ");
        assert_eq!(display_width(&row), 9);
    }

    #[test]
    fn row_format_preserves_lead_or_badge_at_extreme_widths() {
        assert_eq!(format_row(' ', "overflow", None, 1), " ");
        assert_eq!(format_row(' ', "overflow", None, 0), "");
        assert_eq!(format_row(' ', "overflow", Some(""), 3), "  ");
    }

    #[test]
    fn pane_row_is_indented_and_ellipsized_by_terminal_cells() {
        assert_eq!(
            format_pane_row(' ', "very-long-pane", Some(""), 12),
            " very-lo…  "
        );
        let wide = format_pane_row(ARROW_UP, "界界界界", None, 12);
        assert_eq!(wide, "▲界界界界   ");
        assert_eq!(display_width(&wide), 12);
    }

    #[test]
    fn native_list_rows_reserve_component_chrome_and_map_hierarchy() {
        let tab_row = SidebarRow::Tab {
            position: 0,
            name: "very-long-tab-name".to_owned(),
            active: true,
            state: Some(AgentState::Done),
            attention: false,
        };
        let pane_row = SidebarRow::Pane {
            id: 42,
            title: "very-long-pane-name".to_owned(),
            focused: false,
            state: Some(AgentState::Working),
        };

        let tab = native_list_row(&tab_row, ' ', 15);
        let pane = native_list_row(&pane_row, ' ', 15);

        assert_eq!(tab.indentation, 0);
        assert!(tab.selected);
        assert_eq!(display_width(&tab.content), 12);
        assert!(tab.content.ends_with(" "));
        assert_eq!(pane.indentation, 1);
        assert!(!pane.selected);
        assert_eq!(display_width(&pane.content), 10);
        assert!(pane.content.ends_with(" "));
        assert_eq!(tab_row.target(), RowTarget::Tab { position: 0 });
        assert_eq!(pane_row.target(), RowTarget::Pane { id: 42 });
    }

    #[test]
    fn native_list_rows_preserve_badges_at_narrow_widths() {
        let tab_row = SidebarRow::Tab {
            position: 0,
            name: "overflow".to_owned(),
            active: false,
            state: Some(AgentState::Working),
            attention: false,
        };
        let pane_row = SidebarRow::Pane {
            id: 42,
            title: "overflow".to_owned(),
            focused: true,
            state: Some(AgentState::Waiting),
        };

        let tab = native_list_row(&tab_row, ' ', 8);
        let pane = native_list_row(&pane_row, ' ', 8);

        assert_eq!(tab.content, " …  ");
        assert_eq!(pane.content, "  ");
        assert!(pane.selected);
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
    fn parses_and_strictly_validates_acknowledgement_payloads() {
        assert_eq!(
            parse_agent_acknowledgement(
                r#"{"version":1,"pane_id":"terminal_7","session_id":"session-a","updated_at_ms":42}"#,
            ),
            Some(AgentAcknowledgementUpdate {
                pane_id: 7,
                acknowledgement: acknowledgement("session-a", 42),
            })
        );

        for payload in [
            "not json",
            r#"{"version":2,"pane_id":"terminal_7","session_id":"s","updated_at_ms":1}"#,
            r#"{"version":1,"pane_id":"plugin_7","session_id":"s","updated_at_ms":1}"#,
            r#"{"version":1,"pane_id":"terminal_7","session_id":"","updated_at_ms":1}"#,
            r#"{"version":1,"pane_id":"terminal_7","session_id":"s","updated_at_ms":0}"#,
        ] {
            assert_eq!(
                parse_agent_acknowledgement(payload),
                None,
                "payload: {payload}"
            );
        }
    }

    #[test]
    fn focus_observation_round_trips_and_rejects_invalid_panes() {
        let focused_panes = HashSet::from([7, 9]);
        let payload = serialize_agent_focus(&focused_panes).unwrap();
        assert_eq!(parse_agent_focus(&payload), Some(focused_panes));

        assert_eq!(
            parse_agent_focus(r#"{"version":1,"pane_ids":[]}"#),
            Some(HashSet::new())
        );
        for payload in [
            "not json",
            r#"{"version":2,"pane_ids":["terminal_7"]}"#,
            r#"{"version":1,"pane_ids":["plugin_7"]}"#,
            r#"{"version":1,"pane_ids":["terminal_7","terminal_7"]}"#,
        ] {
            assert_eq!(parse_agent_focus(payload), None, "payload: {payload}");
        }
    }

    #[test]
    fn acknowledgement_is_idempotent_and_rejects_older_references() {
        let mut acknowledgements = HashMap::new();
        let update = |session_id: &str, updated_at_ms| AgentAcknowledgementUpdate {
            pane_id: 3,
            acknowledgement: acknowledgement(session_id, updated_at_ms),
        };

        assert!(apply_agent_acknowledgement(
            &mut acknowledgements,
            update("session", 10)
        ));
        assert!(!apply_agent_acknowledgement(
            &mut acknowledgements,
            update("session", 10)
        ));
        assert!(!apply_agent_acknowledgement(
            &mut acknowledgements,
            update("older", 9)
        ));
        assert_eq!(acknowledgements[&3], acknowledgement("session", 10));
    }

    #[test]
    fn matching_done_acknowledgement_renders_idle_only_for_exact_record() {
        let mut records = HashMap::from([(
            4,
            AgentRecord {
                session_id: "session".to_owned(),
                state: AgentState::Done,
                updated_at_ms: 20,
            },
        )]);
        let acknowledgements = HashMap::from([(4, acknowledgement("session", 20))]);

        assert_eq!(
            renderable_agent_state(&records, &acknowledgements, 4),
            Some(AgentState::Idle)
        );
        records.get_mut(&4).unwrap().updated_at_ms = 21;
        assert_eq!(
            renderable_agent_state(&records, &acknowledgements, 4),
            Some(AgentState::Done)
        );
        records.get_mut(&4).unwrap().state = AgentState::Waiting;
        assert_eq!(
            renderable_agent_state(&records, &acknowledgements, 4),
            Some(AgentState::Waiting)
        );
        records.get_mut(&4).unwrap().state = AgentState::Working;
        assert_eq!(
            renderable_agent_state(&records, &acknowledgements, 4),
            Some(AgentState::Working)
        );
    }

    #[test]
    fn newer_status_prunes_superseded_acknowledgement_but_future_one_survives() {
        let mut acknowledgements = HashMap::from([(4, acknowledgement("session", 20))]);
        let newer = AgentRecord {
            session_id: "session".to_owned(),
            state: AgentState::Working,
            updated_at_ms: 21,
        };
        assert!(prune_superseded_agent_acknowledgement(
            &mut acknowledgements,
            4,
            &newer
        ));
        assert!(!acknowledgements.contains_key(&4));

        acknowledgements.insert(4, acknowledgement("future", 30));
        assert!(!prune_superseded_agent_acknowledgement(
            &mut acknowledgements,
            4,
            &newer
        ));
        assert_eq!(acknowledgements[&4], acknowledgement("future", 30));
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
    fn pane_manifest_filters_plugins_and_orders_terminal_layers_and_geometry() {
        let pane = |id, title: &str, x, y, floating, suppressed, plugin| PaneInfo {
            id,
            title: title.to_owned(),
            pane_x: x,
            pane_y: y,
            is_floating: floating,
            is_suppressed: suppressed,
            is_plugin: plugin,
            ..Default::default()
        };
        let manifest = PaneManifest {
            panes: HashMap::from([(
                0,
                vec![
                    pane(8, "suppressed", 0, 0, false, true, false),
                    pane(4, "right", 20, 0, false, false, false),
                    pane(6, "floating", 0, 0, true, false, false),
                    pane(3, "lower", 0, 10, false, false, false),
                    pane(2, "left", 0, 0, false, false, false),
                    pane(1, "plugin", 0, 0, false, false, true),
                ],
            )]),
        };

        let panes = terminal_panes_by_tab(&manifest);
        assert_eq!(
            panes[&0].iter().map(|pane| pane.id).collect::<Vec<_>>(),
            vec![2, 4, 3, 6, 8]
        );
    }

    #[test]
    fn clear_tombstones_are_not_rendered() {
        let records = HashMap::from([(
            4,
            AgentRecord {
                session_id: "session".to_owned(),
                state: AgentState::Clear,
                updated_at_ms: 20,
            },
        )]);
        assert_eq!(renderable_agent_state(&records, &HashMap::new(), 4), None);
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

        let mut acknowledgements = HashMap::from([
            (1, acknowledgement("closed", 1)),
            (2, acknowledgement("open", 2)),
        ]);
        assert!(remove_missing_agent_acknowledgements(
            &mut acknowledgements,
            &pane_tabs
        ));
        assert_eq!(
            acknowledgements,
            HashMap::from([(2, acknowledgement("open", 2))])
        );
        assert!(!remove_missing_agent_acknowledgements(
            &mut acknowledgements,
            &pane_tabs
        ));
    }

    #[test]
    fn adaptive_hierarchy_keeps_single_pane_compact_and_expands_multiple_panes() {
        let tabs = vec![tab(0, "single", true), tab(1, "multiple", false)];
        let panes = HashMap::from([
            (0, vec![terminal_pane(1, "shell", true, false, false, 0, 0)]),
            (
                1,
                vec![
                    terminal_pane(2, "api", false, false, false, 0, 0),
                    terminal_pane(3, "database", true, false, false, 40, 0),
                ],
            ),
        ]);
        let records = HashMap::from([
            (1, agent_record(AgentState::Done)),
            (2, agent_record(AgentState::Working)),
            (3, agent_record(AgentState::Waiting)),
        ]);

        let rows = build_sidebar_rows(&tabs, &panes, &records, &HashMap::new());
        assert_eq!(
            rows,
            vec![
                SidebarRow::Tab {
                    position: 0,
                    name: "single".to_owned(),
                    active: true,
                    state: Some(AgentState::Done),
                    attention: false,
                },
                SidebarRow::Tab {
                    position: 1,
                    name: "multiple".to_owned(),
                    active: false,
                    state: None,
                    attention: false,
                },
                SidebarRow::Pane {
                    id: 2,
                    title: "api".to_owned(),
                    focused: false,
                    state: Some(AgentState::Working),
                },
                SidebarRow::Pane {
                    id: 3,
                    title: "database".to_owned(),
                    focused: false,
                    state: Some(AgentState::Waiting),
                },
            ]
        );
        assert_eq!(active_tab_row(&rows), Some(0));
    }

    #[test]
    fn native_bell_attention_stays_on_tab_while_pane_status_stays_exact() {
        let mut single = tab(0, "single", false);
        single.has_bell_notification = true;
        let mut multiple = tab(1, "multiple", false);
        multiple.has_bell_notification = true;
        let tabs = vec![single, multiple];
        let panes = HashMap::from([
            (
                0,
                vec![terminal_pane(1, "shell", false, false, false, 0, 0)],
            ),
            (
                1,
                vec![
                    terminal_pane(2, "api", false, false, false, 0, 0),
                    terminal_pane(3, "database", false, false, false, 40, 0),
                ],
            ),
        ]);
        let records = HashMap::from([
            (1, agent_record(AgentState::Done)),
            (2, agent_record(AgentState::Waiting)),
        ]);

        let rows = build_sidebar_rows(&tabs, &panes, &records, &HashMap::new());
        assert!(rows[0].has_attention());
        assert_eq!(rows[0].state(), Some(AgentState::Done));
        assert!(rows[1].has_attention());
        assert_eq!(rows[1].state(), None);
        assert!(!rows[2].has_attention());
        assert_eq!(rows[2].state(), Some(AgentState::Waiting));

        assert!(native_list_row(&rows[0], ' ', 15).content.ends_with("  "));
        assert!(native_list_row(&rows[1], ' ', 15).content.ends_with(" "));
        assert!(native_list_row(&rows[2], ' ', 15).content.ends_with(" "));
    }

    #[test]
    fn attention_icon_is_single_cell_and_wins_at_extreme_widths() {
        assert_eq!(display_width(ATTENTION_GLYPH), 1);
        let row = SidebarRow::Tab {
            position: 0,
            name: "narrow".to_owned(),
            active: false,
            state: Some(AgentState::Done),
            attention: true,
        };

        let rendered = native_list_row(&row, ' ', TAB_LIST_CHROME_WIDTH + 2);
        assert_eq!(rendered.content, " ");
        assert_eq!(rendered.state, None);
        assert_eq!(rendered.attention_chars, 1);
    }

    #[test]
    fn zero_pane_tab_remains_one_row_and_empty_title_has_fallback() {
        let tabs = vec![tab(0, "empty", false), tab(1, "two", true)];
        let panes = HashMap::from([(
            1,
            vec![
                terminal_pane(7, "", true, false, false, 0, 0),
                terminal_pane(8, "named", false, false, false, 10, 0),
            ],
        )]);

        let rows = build_sidebar_rows(&tabs, &panes, &HashMap::new(), &HashMap::new());
        assert_eq!(rows.len(), 4);
        assert!(matches!(rows[0], SidebarRow::Tab { position: 0, .. }));
        assert!(matches!(
            &rows[2],
            SidebarRow::Pane { title, .. } if title == "pane 7"
        ));
        assert!(rows[2].is_selected());
        assert!(!rows[3].is_selected());
    }

    #[test]
    fn visible_floating_layer_selects_only_its_focused_pane() {
        let mut active_tab = tab(0, "layers", true);
        active_tab.are_floating_panes_visible = true;
        let panes = HashMap::from([(
            0,
            vec![
                terminal_pane(1, "tiled", true, false, false, 0, 0),
                terminal_pane(2, "floating", true, true, false, 5, 5),
            ],
        )]);

        let rows = build_sidebar_rows(&[active_tab], &panes, &HashMap::new(), &HashMap::new());
        assert!(!rows[1].is_selected());
        assert!(rows[2].is_selected());
    }

    #[test]
    fn newly_focused_done_record_produces_exact_acknowledgement_only() {
        let tabs = vec![tab(0, "active", true), tab(1, "inactive", false)];
        let panes = HashMap::from([
            (
                0,
                vec![terminal_pane(7, "focused", true, false, false, 0, 0)],
            ),
            (1, vec![terminal_pane(8, "other", true, false, false, 0, 0)]),
        ]);
        let mut records = HashMap::from([
            (
                7,
                AgentRecord {
                    session_id: "focused-session".to_owned(),
                    state: AgentState::Done,
                    updated_at_ms: 42,
                },
            ),
            (
                8,
                AgentRecord {
                    session_id: "inactive-session".to_owned(),
                    state: AgentState::Done,
                    updated_at_ms: 43,
                },
            ),
        ]);

        let focused_panes: HashSet<u32> = focused_terminal_pane_ids(&tabs, &panes)
            .into_iter()
            .collect();
        assert_eq!(
            newly_focused_done_acknowledgements(&HashSet::new(), &focused_panes, &records),
            vec![AgentAcknowledgementUpdate {
                pane_id: 7,
                acknowledgement: acknowledgement("focused-session", 42),
            }]
        );
        assert!(
            newly_focused_done_acknowledgements(&focused_panes, &focused_panes, &records)
                .is_empty()
        );
        records.get_mut(&7).unwrap().state = AgentState::Working;
        assert!(
            newly_focused_done_acknowledgements(&HashSet::new(), &focused_panes, &records)
                .is_empty()
        );
        records.get_mut(&7).unwrap().state = AgentState::Waiting;
        assert!(
            newly_focused_done_acknowledgements(&HashSet::new(), &focused_panes, &records)
                .is_empty()
        );
    }

    #[test]
    fn attached_client_focus_overrides_plugin_local_active_tab() {
        let locally_active = tab(0, "plugin-local", true);
        let mut client_focused = tab(1, "client-focused", false);
        client_focused.other_focused_clients = vec![1];
        let tabs = vec![locally_active, client_focused];
        let panes = HashMap::from([
            (0, vec![terminal_pane(7, "local", true, false, false, 0, 0)]),
            (
                1,
                vec![terminal_pane(8, "viewed", true, false, false, 0, 0)],
            ),
        ]);
        let records = HashMap::from([
            (
                7,
                AgentRecord {
                    session_id: "local-session".to_owned(),
                    state: AgentState::Done,
                    updated_at_ms: 42,
                },
            ),
            (
                8,
                AgentRecord {
                    session_id: "viewed-session".to_owned(),
                    state: AgentState::Done,
                    updated_at_ms: 43,
                },
            ),
        ]);

        assert_eq!(
            focused_terminal_pane_ids(&tabs, &panes),
            vec![8],
            "only the pane viewed by an attached Zellij client is acknowledged"
        );
        let focused_panes: HashSet<u32> = focused_terminal_pane_ids(&tabs, &panes)
            .into_iter()
            .collect();
        assert_eq!(
            newly_focused_done_acknowledgements(&HashSet::from([7]), &focused_panes, &records),
            vec![AgentAcknowledgementUpdate {
                pane_id: 8,
                acknowledgement: acknowledgement("viewed-session", 43),
            }]
        );
    }

    #[test]
    fn peer_focus_observation_preserves_done_until_returning_to_completed_pane() {
        let mut state = State {
            tabs: vec![tab(0, "first", true), tab(1, "second", false)],
            terminal_panes: HashMap::from([
                (0, vec![terminal_pane(7, "first", true, false, false, 0, 0)]),
                (
                    1,
                    vec![terminal_pane(8, "second", true, false, false, 0, 0)],
                ),
            ]),
            focused_terminal_panes: Some(HashSet::from([7])),
            ..State::default()
        };

        assert!(state.accept_agent_status(AgentStatusUpdate {
            pane_id: 7,
            record: AgentRecord {
                session_id: "completed-elsewhere".to_owned(),
                state: AgentState::Done,
                updated_at_ms: 44,
            },
        }));
        assert!(state.agent_acknowledgements.is_empty());

        assert!(!state.accept_focus_observation(HashSet::from([8])));
        assert!(state.agent_acknowledgements.is_empty());

        assert!(state.accept_focus_observation(HashSet::from([7])));
        assert_eq!(
            state.agent_acknowledgements.get(&7),
            Some(&acknowledgement("completed-elsewhere", 44))
        );
    }

    #[test]
    fn flattened_window_follows_active_tab_after_prior_pane_children() {
        let tabs = vec![tab(0, "many", false), tab(1, "active", true)];
        let panes = HashMap::from([(
            0,
            vec![
                terminal_pane(1, "one", false, false, false, 0, 0),
                terminal_pane(2, "two", false, false, false, 10, 0),
            ],
        )]);
        let sidebar_rows = build_sidebar_rows(&tabs, &panes, &HashMap::new(), &HashMap::new());

        assert_eq!(active_tab_row(&sidebar_rows), Some(3));
        assert_eq!(visible_window(sidebar_rows.len(), Some(3), 0, 2), 2);
    }

    #[test]
    fn row_targets_distinguish_tab_switches_from_pane_focus() {
        let tab_row = SidebarRow::Tab {
            position: 4,
            name: "tab".to_owned(),
            active: false,
            state: None,
            attention: false,
        };
        let pane_row = SidebarRow::Pane {
            id: 42,
            title: "pane".to_owned(),
            focused: false,
            state: None,
        };

        assert_eq!(tab_row.target(), RowTarget::Tab { position: 4 });
        assert_eq!(pane_row.target(), RowTarget::Pane { id: 42 });
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
        let source_acknowledgements = HashMap::from([(2, acknowledgement("two", 20))]);
        let payload = serialize_agent_snapshot(&source, &source_acknowledgements).unwrap();
        let mut destination = HashMap::from([(
            1,
            AgentRecord {
                session_id: "newer".to_owned(),
                state: AgentState::Waiting,
                updated_at_ms: 11,
            },
        )]);

        let mut acknowledgements = HashMap::new();
        assert!(apply_agent_snapshot(
            &mut destination,
            &mut acknowledgements,
            &payload
        ));
        assert_eq!(destination[&1].session_id, "newer");
        assert_eq!(destination[&1].state, AgentState::Waiting);
        assert_eq!(destination[&2], source[&2]);
        assert_eq!(acknowledgements, source_acknowledgements);
        assert!(!apply_agent_snapshot(
            &mut destination,
            &mut acknowledgements,
            &payload
        ));
        assert!(!apply_agent_snapshot(
            &mut destination,
            &mut acknowledgements,
            r#"{"version":2,"records":[]}"#
        ));
    }

    #[test]
    fn snapshot_recovers_acknowledgement_before_matching_status() {
        let source_records = HashMap::from([(
            7,
            AgentRecord {
                session_id: "session".to_owned(),
                state: AgentState::Done,
                updated_at_ms: 42,
            },
        )]);
        let source_acknowledgements = HashMap::from([(7, acknowledgement("session", 42))]);
        let payload = serialize_agent_snapshot(&source_records, &source_acknowledgements).unwrap();
        let mut records = HashMap::new();
        let mut acknowledgements = HashMap::from([(7, acknowledgement("future", 50))]);

        assert!(apply_agent_snapshot(
            &mut records,
            &mut acknowledgements,
            &payload
        ));
        assert_eq!(records, source_records);
        assert_eq!(acknowledgements[&7], acknowledgement("future", 50));

        let acknowledgement_payload =
            serialize_agent_acknowledgement(9, &acknowledgement("before-status", 60)).unwrap();
        let update = parse_agent_acknowledgement(&acknowledgement_payload).unwrap();
        assert!(apply_agent_acknowledgement(&mut acknowledgements, update));
        assert_eq!(renderable_agent_state(&records, &acknowledgements, 9), None);
        records.insert(
            9,
            AgentRecord {
                session_id: "before-status".to_owned(),
                state: AgentState::Done,
                updated_at_ms: 60,
            },
        );
        assert_eq!(
            renderable_agent_state(&records, &acknowledgements, 9),
            Some(AgentState::Idle)
        );
    }

    #[test]
    fn cache_round_trip_restores_status_and_acknowledgement() {
        let cache_dir = temp_directory("cache-round-trip");
        let records = HashMap::from([(
            7,
            AgentRecord {
                session_id: "completed".to_owned(),
                state: AgentState::Done,
                updated_at_ms: 42,
            },
        )]);
        let acknowledgements = HashMap::from([(7, acknowledgement("completed", 42))]);

        assert!(persist_agent_cache(
            &cache_dir,
            100,
            9,
            &records,
            &acknowledgements
        ));
        let mut restored_records = HashMap::new();
        let mut restored_acknowledgements = HashMap::new();
        assert!(restore_agent_cache(
            &cache_dir,
            100,
            &mut restored_records,
            &mut restored_acknowledgements
        ));
        assert_eq!(restored_records, records);
        assert_eq!(restored_acknowledgements, acknowledgements);

        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[test]
    fn cache_restore_is_server_scoped_and_skips_invalid_entries() {
        let cache_dir = temp_directory("cache-filtering");
        let records = HashMap::from([(1, agent_record(AgentState::Working))]);
        assert!(persist_agent_cache(
            &cache_dir,
            200,
            1,
            &records,
            &HashMap::new()
        ));
        assert!(persist_agent_cache(
            &cache_dir,
            201,
            2,
            &HashMap::from([(2, agent_record(AgentState::Waiting))]),
            &HashMap::new()
        ));
        std::fs::write(cache_dir.join("agent-status-200-not-a-number.json"), "{}").unwrap();
        std::fs::write(cache_dir.join("agent-status-200-3.json"), "not-json").unwrap();
        let oversized = std::fs::File::create(cache_dir.join("agent-status-200-4.json")).unwrap();
        oversized.set_len(AGENT_CACHE_MAX_BYTES as u64 + 1).unwrap();

        let mut restored_records = HashMap::new();
        let mut restored_acknowledgements = HashMap::new();
        assert!(restore_agent_cache(
            &cache_dir,
            200,
            &mut restored_records,
            &mut restored_acknowledgements
        ));
        assert_eq!(restored_records, records);
        assert!(restored_acknowledgements.is_empty());

        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[test]
    fn host_snapshot_clear_reconciles_cache_without_overwriting_newer_live_state() {
        let mut records = HashMap::from([(
            5,
            AgentRecord {
                session_id: "session".to_owned(),
                state: AgentState::Done,
                updated_at_ms: 10,
            },
        )]);
        let mut acknowledgements = HashMap::from([(5, acknowledgement("session", 10))]);
        let clear = serialize_agent_snapshot(
            &HashMap::from([(
                5,
                AgentRecord {
                    session_id: "session".to_owned(),
                    state: AgentState::Clear,
                    updated_at_ms: 20,
                },
            )]),
            &HashMap::new(),
        )
        .unwrap();
        assert!(apply_agent_snapshot(
            &mut records,
            &mut acknowledgements,
            &clear
        ));
        assert_eq!(records[&5].state, AgentState::Clear);
        assert!(acknowledgements.is_empty());

        records.insert(
            5,
            AgentRecord {
                session_id: "next".to_owned(),
                state: AgentState::Working,
                updated_at_ms: 30,
            },
        );
        assert!(!apply_agent_snapshot(
            &mut records,
            &mut acknowledgements,
            &clear
        ));
        assert_eq!(records[&5].session_id, "next");
        assert_eq!(records[&5].state, AgentState::Working);
    }

    #[test]
    fn badge_is_right_aligned_and_preserved_when_name_is_long() {
        assert_eq!(format_row(' ', "work", Some(""), 10), " work    ");
        assert_eq!(
            format_row(' ', "very-long-name", Some(""), 10),
            " very-…  "
        );
        assert_eq!(format_row(' ', "x", Some(""), 1), "");
        assert_eq!(format_row(' ', "x", Some(""), 2), " ");
        assert_eq!(format_row(' ', "界界界界", Some(""), 10), " 界界…   ");
        assert_eq!(display_width(" 界界…   "), 10);
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
        let content = " 1 work  ";
        let item = color_agent_badge_item(
            NestedListItem::new(content).indent(1),
            AgentState::Working,
            content.chars().count(),
            1,
            0,
            1,
        )
        .selected();
        let serialized = item.serialize();

        assert!(serialized.starts_with("|x$8$"));
    }
}
