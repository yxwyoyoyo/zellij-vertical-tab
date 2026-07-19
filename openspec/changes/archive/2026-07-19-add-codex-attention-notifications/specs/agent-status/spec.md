## ADDED Requirements

### Requirement: Native attention notification
The Codex TUI SHALL request native terminal attention for completed turns and approval requests, using BEL so Zellij can flash an active tab or retain attention for an inactive tab independently of Codex's terminal-focus detection.

#### Scenario: A Codex turn completes
- **WHEN** Codex emits `agent-turn-complete`
- **THEN** Codex emits BEL and Zellij records native bell attention for the owning tab

#### Scenario: A Codex session requests approval
- **WHEN** Codex emits `approval-requested`
- **THEN** Codex emits BEL and Zellij records native bell attention for the owning tab

#### Scenario: Codex remains terminal-focused after a Zellij tab switch
- **WHEN** either configured notification event occurs after the user moved to another Zellij tab but Codex still observes terminal focus
- **THEN** Codex emits BEL because the notification condition is `always`
- **AND** Zellij retains attention for the inactive owning tab

#### Scenario: A completed turn asks for user input
- **WHEN** Codex finishes a turn whose answer asks the user a question
- **THEN** the event uses the completed-turn notification because Codex exposes no separate structured waiting-for-answer event
