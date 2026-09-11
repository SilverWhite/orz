//! ACAF ticket flow — control/action/network/command ticket lifecycles —
//! batch N1 of the controller split second round
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29 §3.5).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use orz_assurance::EventType;
use orz_assurance::acaf::TicketKind;

use crate::controller::{AgentLoopController, AgentLoopError, EventWriter, TicketGate};

impl AgentLoopController {
    /// Journal the terminal outcome of a ticket lifecycle (consumed /
    /// rejected) — the shared tail of the control and action paths.
    async fn journal_ticket_outcome(
        &self,
        writer: &mut EventWriter<'_>,
        outcome: &crate::acaf::TicketOutcome,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> Result<(), AgentLoopError> {
        match outcome {
            crate::acaf::TicketOutcome::Consumed { .. } => {
                writer
                    .record(
                        EventType::ControlTicketConsumed,
                        crate::acaf::consumed_payload(outcome, now),
                    )
                    .await?;
            }
            _ => {
                writer
                    .record(
                        EventType::ControlTicketRejected,
                        crate::acaf::rejected_payload(outcome, now),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    /// ACAF (ADR-0011 §4.2/§4.6): sign → journal issued → verify → journal
    /// consumed|rejected for ONE ticket. Shadow mode: every failure path
    /// journals `control_ticket_rejected` (first reject code, or
    /// `signer_unreachable`) and the event still proceeds. Unticketed paths
    /// (no client / no goal yet) journal nothing. `resolved_target` (Slice 2
    /// first phase) binds the parsed real target — action kinds only;
    /// control kinds pass None. Slice 2 fail-closed (2026-08-13): a
    /// rejected outcome returns `TicketGate::Blocked` — the caller refuses
    /// the event/action (D-14/D-15/D-16).
    async fn ticket_flow(
        &self,
        writer: &mut EventWriter<'_>,
        kind: TicketKind,
        activation_id: Option<String>,
        args: &serde_json::Value,
        resolved_target: Option<String>,
    ) -> Result<TicketGate, AgentLoopError> {
        let Some(acaf) = &self.acaf else {
            // D-15 fail-closed: an unconfigured fabric is caught at run
            // start (startup fail-fast). If ticket_flow is reached with fail-closed
            // enabled but without a signer, refuse immediately.
            if self.acaf_fail_closed {
                let now = chrono::Utc::now();
                return self
                    .fail_closed_refusal(
                        writer,
                        kind,
                        orz_assurance::acaf::RejectCode::SignerUnreachable,
                        "ACAF fail-closed is enabled but no signer client is configured"
                            .to_string(),
                        &now,
                    )
                    .await;
            }
            return Ok(TicketGate::Proceed);
        };
        let Some((goal_digest, goal_version)) = self.goal_binding_snapshot() else {
            // D-15 (2026-08-13): no goal context → action/control
            // tickets cannot bind check 4. Shadow: silent skip
            // (registered boundary); fail-closed: journal
            // `missing_goal_context` + refuse.
            if self.acaf_fail_closed {
                let now = chrono::Utc::now();
                return self
                    .fail_closed_refusal(
                        writer,
                        kind,
                        orz_assurance::acaf::RejectCode::MissingGoalContext,
                        "goal digest not pinned (run goal must be set before ticketed events)"
                            .to_string(),
                        &now,
                    )
                    .await;
            }
            return Ok(TicketGate::Proceed);
        };
        let mut client = acaf.lock().await;
        let session_id = self
            .session_id
            .clone()
            .unwrap_or_else(|| writer.run_id().to_string());
        let canonical = crate::acaf::canonical_arguments_digest(kind, args);
        // P0-0x S2: the orientation trigger rides the ticket — the signer
        // picks the built-in template digest by it, and check 2 compares
        // against the same one. Absent for every other ticket kind.
        let trigger = args.get("trigger").and_then(serde_json::Value::as_str);
        let now = chrono::Utc::now();
        // Goal/policy wiring (2026-08-12): the live goal version and policy
        // revision — a mismatch against the client's cached session
        // re-derives K_session (accepted continue / policy bump → old
        // tickets die, ADR-0011 决策 5).
        let outcome = match client
            .ensure_initialized(
                &session_id,
                "main",
                goal_version,
                &goal_digest,
                self.policy_revision(),
            )
            .await
        {
            Ok(()) => match client
                .sign_ticket(
                    kind,
                    activation_id.clone(),
                    &canonical,
                    resolved_target.clone(),
                    trigger,
                )
                .await
            {
                Ok(ticket) => {
                    writer
                        .record(
                            EventType::ControlTicketIssued,
                            crate::acaf::issued_payload(&ticket),
                        )
                        .await?;
                    match client
                        .verify_and_consume(
                            &ticket,
                            &canonical,
                            activation_id.clone(),
                            resolved_target,
                            trigger,
                        )
                        .await
                    {
                        Ok(outcome) => outcome,
                        Err(e) => crate::acaf::TicketOutcome::SignerUnreachable {
                            kind,
                            detail: e.to_string(),
                        },
                    }
                }
                Err(e) => crate::acaf::TicketOutcome::SignerUnreachable {
                    kind,
                    detail: e.to_string(),
                },
            },
            Err(e) => crate::acaf::TicketOutcome::SignerUnreachable {
                kind,
                detail: e.to_string(),
            },
        };
        self.journal_ticket_outcome(writer, &outcome, &now).await?;
        Ok(match crate::acaf::ticket_outcome_reject(&outcome) {
            None => TicketGate::Proceed,
            Some((code, detail)) => {
                if self.acaf_fail_closed {
                    TicketGate::Blocked { code, detail }
                } else {
                    TicketGate::Proceed
                }
            }
        })
    }

    /// ACAF Slice 1 (ADR-0011 §4.2/§4.6) — the control-event ticket
    /// lifecycle (orientation / disposition / close / goal revision).
    /// Control kinds carry no resolved target.
    pub(crate) async fn acaf_control_event(
        &self,
        writer: &mut EventWriter<'_>,
        kind: TicketKind,
        activation_id: Option<String>,
        args: &serde_json::Value,
    ) -> Result<TicketGate, AgentLoopError> {
        self.ticket_flow(writer, kind, activation_id, args, None)
            .await
    }

    /// Rejection for an unticketable action target (D7): no ticket was
    /// issued, so the `control_ticket_rejected` event carries a null
    /// ticket_id with `target_mismatch` (mirrors the signer_unreachable
    /// null-ticket_id precedent). Journaled in BOTH modes; fail-closed
    /// additionally returns `Blocked`.
    async fn shadow_action_rejection(
        &self,
        writer: &mut EventWriter<'_>,
        kind: TicketKind,
        detail: String,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> Result<TicketGate, AgentLoopError> {
        self.journal_ticket_outcome(
            writer,
            &crate::acaf::TicketOutcome::Rejected {
                ticket_id: None,
                kind,
                code: orz_assurance::acaf::RejectCode::TargetMismatch,
                detail: detail.clone(),
            },
            now,
        )
        .await?;
        if self.acaf_fail_closed {
            Ok(TicketGate::Blocked {
                code: orz_assurance::acaf::RejectCode::TargetMismatch,
                detail,
            })
        } else {
            Ok(TicketGate::Proceed)
        }
    }

    /// D-14/D-15 fail-closed refusal (2026-08-13): a PRE-SIGNING refusal —
    /// a required target argument is missing/empty or a dependency
    /// (snapshot store / goal context) is absent. Shadow mode stays SILENT
    /// (registered boundary: these paths journal nothing in the shadow
    /// ledger); fail-closed journals `control_ticket_rejected` with a null
    /// ticket_id and returns `Blocked` (the tool/event is not executed).
    async fn fail_closed_refusal(
        &self,
        writer: &mut EventWriter<'_>,
        kind: TicketKind,
        code: orz_assurance::acaf::RejectCode,
        detail: String,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> Result<TicketGate, AgentLoopError> {
        if !self.acaf_fail_closed {
            return Ok(TicketGate::Proceed);
        }
        self.journal_ticket_outcome(
            writer,
            &crate::acaf::TicketOutcome::Rejected {
                ticket_id: None,
                kind,
                code,
                detail: detail.clone(),
            },
            now,
        )
        .await?;
        Ok(TicketGate::Blocked { code, detail })
    }

    /// Read the goal binding snapshot (digest + version) WITHOUT holding
    /// the `std::sync::MutexGuard` across an await — the fail-closed
    /// missing-goal-context refusal journals (awaits) and the guard is not
    /// `Send` (review D-15 2026-08-13, tokio::spawn test compile).
    fn goal_binding_snapshot(&self) -> Option<(String, u64)> {
        let g = self.goal_context.lock().unwrap();
        g.digest.clone().map(|digest| (digest, g.version))
    }

    /// ACAF Slice 2 first phase (2026-08-12) — action-ticket lifecycle for
    /// an external-effect tool call. The live target is resolved and bound
    /// (`resolved_target_sha256`, check 5b), the canonical arguments bind the
    /// resolved path + operation + content digest (D5), and the consumption
    /// point RE-RESOLVES and re-derives both digests from the live arguments
    /// — never trusting the ticket's own values. Shadow mode (D7): every
    /// failure path journals `control_ticket_rejected` (unticketable target
    /// → null ticket_id + `target_mismatch`) and the tool proceeds —
    /// the shadow ledger IS the journal events. Fail-closed flips at the
    /// full Slice 2 milestone.
    pub(crate) async fn acaf_action_event(
        &self,
        writer: &mut EventWriter<'_>,
        tool: &str,
        tc_args: &serde_json::Value,
        activation_id: Option<String>,
    ) -> Result<TicketGate, AgentLoopError> {
        let Some(kind) = crate::acaf::action_kind_for_tool(tool) else {
            return Ok(TicketGate::Proceed);
        };
        // Unticketed paths journal nothing (zero behaviour change when ACAF
        // is unconfigured — same gate as the control events). This MUST
        // precede the network/command dispatch below: an unconfigured
        // fabric would otherwise journal shadow rejections for invalid
        // URLs/commands.
        if self.acaf.is_none() {
            if self.acaf_fail_closed {
                let now = chrono::Utc::now();
                return self
                    .fail_closed_refusal(
                        writer,
                        kind,
                        orz_assurance::acaf::RejectCode::SignerUnreachable,
                        "ACAF fail-closed is enabled but no signer client is configured"
                            .to_string(),
                        &now,
                    )
                    .await;
            }
            return Ok(TicketGate::Proceed);
        }
        // Slice 2 full phase (2026-08-12): the network and command branches
        // resolve their own target shapes (canonical URL / argv-cwd-env
        // triple) and share the `run_action_ticket` lifecycle; the proven
        // file_write path below stays untouched.
        if kind == TicketKind::NetworkV1 {
            return self
                .acaf_network_event(writer, tool, tc_args, activation_id)
                .await;
        }
        if kind == TicketKind::CommandExecV1 {
            // `run_tests` never reaches this generic path (the
            // `run_host_tool` branch handles it with the host-owned fixed
            // command BEFORE ToolStarted); `run_terminal_cmd` binds the
            // model-supplied shell command here.
            return self
                .acaf_command_event(writer, tool, tc_args, activation_id)
                .await;
        }
        // Defensive: no snapshot store → no worktree base → the real target
        // cannot be resolved; skip the ticket (production always carries the
        // store — the run_host_tool snapshot block uses the same source).
        // D-15 (2026-08-13): fail-closed turns the silent skip into a hard
        // refusal (`missing_snapshot_store`); shadow stays silent.
        let now = chrono::Utc::now();
        let Some(store) = &self.snapshot_store else {
            return self
                .fail_closed_refusal(
                    writer,
                    kind,
                    orz_assurance::acaf::RejectCode::MissingSnapshotStore,
                    "no snapshot store configured — file_write target cannot be resolved"
                        .to_string(),
                    &now,
                )
                .await;
        };
        let worktree = store.worktree();
        // Review P2-7 (2026-08-12): HOMEDRIVE+HOMEPATH join is the dirs
        // crate fallback shellexpand uses — a session without USERPROFILE
        // but with HOMEDRIVE/HOMEPATH still expands `~` identically.
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .or_else(|| {
                std::env::var_os("HOMEDRIVE").and_then(|drive| {
                    std::env::var_os("HOMEPATH").map(|path| {
                        let mut joined = drive;
                        joined.push(path);
                        joined
                    })
                })
            })
            .map(std::path::PathBuf::from);
        let Some(file_path) = tc_args.get("file_path").and_then(serde_json::Value::as_str) else {
            // D-14 (2026-08-13): missing/empty file_path → hard refusal in
            // fail-closed (`missing_target_argument`, null ticket_id);
            // shadow mode keeps the registered silent skip.
            return self
                .fail_closed_refusal(
                    writer,
                    kind,
                    orz_assurance::acaf::RejectCode::MissingTargetArgument,
                    "file_path argument missing or empty".to_string(),
                    &now,
                )
                .await;
        };
        // D-14 review fix (2026-08-13): an EMPTY file_path is a missing
        // target argument — fail-closed surfaces `missing_target_argument`
        // (same code as the missing case); shadow keeps the registered
        // resolver-error ledger (`target_mismatch`).
        if self.acaf_fail_closed && file_path.trim().is_empty() {
            return self
                .fail_closed_refusal(
                    writer,
                    kind,
                    orz_assurance::acaf::RejectCode::MissingTargetArgument,
                    "file_path argument missing or empty".to_string(),
                    &now,
                )
                .await;
        }
        // Resolve AND reparse-scan in one call (review D1-1 2026-08-12: the
        // scan runs on the UN-FOLDED candidate, so a `<junction>\..` spelling
        // cannot hide the link).
        let target = match orz_assurance::acaf::target::resolve_action_path_checked(
            worktree,
            file_path,
            home.as_deref(),
        ) {
            Ok(t) => t,
            Err(e) => {
                // Shadow rejection for an unticketable target (D7): no
                // ticket exists, so the rejected event carries a null
                // ticket_id.
                return self
                    .shadow_action_rejection(
                        writer,
                        kind,
                        format!("target resolution failed: {e}"),
                        &now,
                    )
                    .await;
            }
        };
        // Case/short-name normalisation when the target exists (a new file
        // keeps its lexical spelling — `dunce::canonicalize` fails on it).
        let effective = orz_assurance::acaf::target::canonicalize_if_exists(&target);
        let resolved_digest = orz_assurance::acaf::target::resolved_target_digest(&effective);
        let effective_str = effective.to_string_lossy().into_owned();
        // Review P1-1 (2026-08-12): the operation classifies against the
        // REAL target state (missing/empty → create, else modify — an empty
        // `old_string` on an existing non-empty file is a full overwrite).
        let operation = crate::acaf::file_write_operation(&effective, tc_args);
        let canonical_args =
            crate::acaf::file_write_canonical_args(tool, &effective_str, operation, tc_args);
        let canonical_digest = crate::acaf::canonical_arguments_digest(kind, &canonical_args);
        let Some((goal_digest, goal_version)) = self.goal_binding_snapshot() else {
            // D-15 (2026-08-13): same missing-goal-context refusal as
            // the shared ticket_flow path.
            if self.acaf_fail_closed {
                return self
                    .fail_closed_refusal(
                        writer,
                        kind,
                        orz_assurance::acaf::RejectCode::MissingGoalContext,
                        "goal digest not pinned".to_string(),
                        &now,
                    )
                    .await;
            }
            return Ok(TicketGate::Proceed);
        };
        let session_id = self
            .session_id
            .clone()
            .unwrap_or_else(|| writer.run_id().to_string());
        // Sign (lock scope 1 — review P2-6 2026-08-12: the mutex covers only
        // the signer RPCs, NOT the re-resolution below; the RPC may self-heal
        // with a signer respawn and must not block later control tickets).
        let signed = {
            let Some(acaf) = &self.acaf else {
                return Ok(TicketGate::Proceed);
            };
            let mut client = acaf.lock().await;
            match client
                .ensure_initialized(
                    &session_id,
                    "main",
                    goal_version,
                    &goal_digest,
                    self.policy_revision(),
                )
                .await
            {
                Ok(()) => match client
                    .sign_ticket(
                        kind,
                        activation_id.clone(),
                        &canonical_digest,
                        Some(resolved_digest.clone()),
                        // Action kinds carry no template (check 2 is
                        // orientation-only) — no trigger on this path.
                        None,
                    )
                    .await
                {
                    Ok(t) => Ok(t),
                    Err(e) => Err(crate::acaf::TicketOutcome::SignerUnreachable {
                        kind,
                        detail: e.to_string(),
                    }),
                },
                Err(e) => Err(crate::acaf::TicketOutcome::SignerUnreachable {
                    kind,
                    detail: e.to_string(),
                }),
            }
        };
        let ticket = match signed {
            Ok(t) => t,
            Err(outcome) => {
                self.journal_ticket_outcome(writer, &outcome, &now).await?;
                return match crate::acaf::ticket_outcome_reject(&outcome) {
                    None => Ok(TicketGate::Proceed),
                    Some((code, detail)) => Ok(if self.acaf_fail_closed {
                        TicketGate::Blocked { code, detail }
                    } else {
                        TicketGate::Proceed
                    }),
                };
            }
        };
        writer
            .record(
                EventType::ControlTicketIssued,
                crate::acaf::issued_payload(&ticket),
            )
            .await?;
        // Verify: RE-RESOLVE the live target and re-derive both digests —
        // a symlink swapped in between sign and verify, a file created
        // between the two operation probes (P1-1), or any drift of the live
        // arguments hits `target_mismatch` (check 5/5b).
        let live_outcome = match orz_assurance::acaf::target::resolve_action_path_checked(
            worktree,
            file_path,
            home.as_deref(),
        ) {
            Ok(target2) => {
                let effective2 = orz_assurance::acaf::target::canonicalize_if_exists(&target2);
                let live_target_digest =
                    orz_assurance::acaf::target::resolved_target_digest(&effective2);
                let live_effective_str = effective2.to_string_lossy().into_owned();
                let live_operation = crate::acaf::file_write_operation(&effective2, tc_args);
                let live_canonical = crate::acaf::file_write_canonical_args(
                    tool,
                    &live_effective_str,
                    live_operation,
                    tc_args,
                );
                let live_canonical_digest =
                    crate::acaf::canonical_arguments_digest(kind, &live_canonical);
                // Lock scope 2 — verify only.
                let Some(acaf) = &self.acaf else {
                    return Ok(TicketGate::Proceed);
                };
                let mut client = acaf.lock().await;
                client
                    .verify_and_consume(
                        &ticket,
                        &live_canonical_digest,
                        activation_id.clone(),
                        Some(live_target_digest),
                        None,
                    )
                    .await
            }
            Err(e) => Ok(crate::acaf::TicketOutcome::Rejected {
                ticket_id: Some(ticket.ticket_id.clone()),
                kind,
                code: orz_assurance::acaf::RejectCode::TargetMismatch,
                detail: format!("live target resolution failed: {e}"),
            }),
        };
        let outcome = match live_outcome {
            Ok(outcome) => outcome,
            // Review fix (2026-08-13): do NOT journal inside the arm — the
            // shared journal below emits exactly ONE terminal event per
            // ticket (the fail-closed slice originally journaled this
            // path twice).
            Err(e) => crate::acaf::TicketOutcome::SignerUnreachable {
                kind,
                detail: e.to_string(),
            },
        };
        self.journal_ticket_outcome(writer, &outcome, &now).await?;
        match crate::acaf::ticket_outcome_reject(&outcome) {
            None => Ok(TicketGate::Proceed),
            Some((code, detail)) => Ok(if self.acaf_fail_closed {
                TicketGate::Blocked { code, detail }
            } else {
                TicketGate::Proceed
            }),
        }
    }

    /// ACAF Slice 2 full phase (2026-08-12) — `network_v1` action tickets
    /// for the URL-carrying network tools (`web_fetch` / `browser_read`).
    /// The live target is the canonical http(s) URL (`resolve_network_url`
    /// — scheme/host normalisation, default-port removal, fragment drop,
    /// userinfo refusal), the canonical arguments bind the tool + canonical
    /// URL, and the consumption point recomputes the canonical object from
    /// the ORIGINAL URL argument — never trusting the ticket's face values
    /// (check 5/5b). For a pure string target this recompute is
    /// deterministic (no external state to drift) — the TOCTOU detection
    /// strength of file_write's FS re-read does not apply here (review
    /// D1-1, 2026-08-12). Shadow mode: an unticketable URL journals
    /// `control_ticket_rejected` (null ticket_id + target_mismatch) and the
    /// tool proceeds.
    async fn acaf_network_event(
        &self,
        writer: &mut EventWriter<'_>,
        tool: &str,
        tc_args: &serde_json::Value,
        activation_id: Option<String>,
    ) -> Result<TicketGate, AgentLoopError> {
        if self.acaf.is_none() {
            return Ok(TicketGate::Proceed);
        }
        let Some(url) = tc_args.get("url").and_then(serde_json::Value::as_str) else {
            // D-14 (2026-08-13): missing/empty url → hard refusal in
            // fail-closed (`missing_target_argument`); shadow keeps the
            // registered silent skip.
            let now = chrono::Utc::now();
            return self
                .fail_closed_refusal(
                    writer,
                    TicketKind::NetworkV1,
                    orz_assurance::acaf::RejectCode::MissingTargetArgument,
                    "url argument missing or empty".to_string(),
                    &now,
                )
                .await;
        };
        let url = url.to_string();
        // D-14 review fix (2026-08-13): an EMPTY/whitespace url is a
        // missing target argument in fail-closed (`missing_target_argument`,
        // same as the missing case); shadow keeps the registered
        // `target_mismatch` ledger (the resolver treats it as Empty).
        if self.acaf_fail_closed && url.trim().is_empty() {
            let now = chrono::Utc::now();
            return self
                .fail_closed_refusal(
                    writer,
                    TicketKind::NetworkV1,
                    orz_assurance::acaf::RejectCode::MissingTargetArgument,
                    "url argument missing or empty".to_string(),
                    &now,
                )
                .await;
        }
        // Review P1-1 (2026-08-12, three-agent review): the URL target needs
        // NO worktree-relative resolution — a snapshot_store gate here was
        // an error copy of the file path branch and would have created an
        // invisible unticketed channel (configured ACAF + missing store →
        // network tool runs with zero ticket events).
        let kind = TicketKind::NetworkV1;
        let now = chrono::Utc::now();
        let canonical_url = match orz_assurance::acaf::target::resolve_network_url(&url) {
            Ok(u) => u,
            Err(e) => {
                return self
                    .shadow_action_rejection(
                        writer,
                        kind,
                        format!("network target resolution failed: {e}"),
                        &now,
                    )
                    .await;
            }
        };
        let canonical_args = crate::acaf::network_canonical_args(tool, &canonical_url);
        let target_digest = orz_assurance::acaf::target::network_target_digest(&canonical_url);
        let tool_owned = tool.to_string();
        self.run_action_ticket(
            writer,
            kind,
            canonical_args,
            target_digest,
            activation_id,
            move || {
                let canonical_url = orz_assurance::acaf::target::resolve_network_url(&url)
                    .map_err(|e| e.to_string())?;
                let canonical = crate::acaf::network_canonical_args(&tool_owned, &canonical_url);
                let digest = orz_assurance::acaf::target::network_target_digest(&canonical_url);
                Ok((canonical, digest))
            },
        )
        .await
    }

    /// ACAF Slice 2 full phase (2026-08-12) — `command_exec_v1` for the
    /// model-supplied shell tool `run_terminal_cmd`. The canonical argv is
    /// the trimmed command string (the shell parses it — the canonical form
    /// is the whole command), the cwd is the session worktree, and the
    /// environment digest is over an empty list (the terminal backend's
    /// process env is host-owned and not model-controlled — registered
    /// boundary). The target digest binds the argv/cwd/env triple
    /// (design §3.3), re-derived at the consumption point.
    async fn acaf_command_event(
        &self,
        writer: &mut EventWriter<'_>,
        tool: &str,
        tc_args: &serde_json::Value,
        activation_id: Option<String>,
    ) -> Result<TicketGate, AgentLoopError> {
        if self.acaf.is_none() {
            return Ok(TicketGate::Proceed);
        }
        let Some(command) = tc_args.get("command").and_then(serde_json::Value::as_str) else {
            // D-14 (2026-08-13): missing command → hard refusal in
            // fail-closed; shadow keeps the registered silent skip.
            let now = chrono::Utc::now();
            return self
                .fail_closed_refusal(
                    writer,
                    TicketKind::CommandExecV1,
                    orz_assurance::acaf::RejectCode::MissingTargetArgument,
                    "command argument missing or empty".to_string(),
                    &now,
                )
                .await;
        };
        let command = command.trim().to_string();
        let kind = TicketKind::CommandExecV1;
        let now = chrono::Utc::now();
        if command.is_empty() {
            // Empty command: shadow keeps the registered target_mismatch
            // ledger; fail-closed surfaces D-14's missing_target_argument
            // and refuses.
            if self.acaf_fail_closed {
                return self
                    .fail_closed_refusal(
                        writer,
                        kind,
                        orz_assurance::acaf::RejectCode::MissingTargetArgument,
                        "command argument missing or empty".to_string(),
                        &now,
                    )
                    .await;
            }
            return self
                .shadow_action_rejection(
                    writer,
                    kind,
                    "command target resolution failed: empty command".to_string(),
                    &now,
                )
                .await;
        }
        let Some(store) = &self.snapshot_store else {
            // D-15 (2026-08-13): the command cwd binds the worktree — no
            // store → no cwd → fail-closed refuses; shadow stays silent.
            return self
                .fail_closed_refusal(
                    writer,
                    kind,
                    orz_assurance::acaf::RejectCode::MissingSnapshotStore,
                    "no snapshot store configured — command cwd cannot be bound".to_string(),
                    &now,
                )
                .await;
        };
        let cwd = store.worktree().to_string_lossy().into_owned();
        let env_sha = crate::acaf::command_env_sha256(&[]);
        let argv = vec![command.clone()];
        let canonical_args = crate::acaf::command_exec_canonical_args(tool, &argv, &cwd, &env_sha);
        let target_digest = crate::acaf::command_exec_target_digest(&argv, &cwd, &env_sha);
        let tool_owned = tool.to_string();
        self.run_action_ticket(
            writer,
            kind,
            canonical_args,
            target_digest,
            activation_id,
            move || {
                let canonical =
                    crate::acaf::command_exec_canonical_args(&tool_owned, &argv, &cwd, &env_sha);
                let digest = crate::acaf::command_exec_target_digest(&argv, &cwd, &env_sha);
                Ok((canonical, digest))
            },
        )
        .await
    }

    /// ACAF Slice 2 full phase (2026-08-12) — `command_exec_v1` for the
    /// host-owned `run_tests` fixed command. The canonical argv is the
    /// host's fixed command (the model supplies no argv), the cwd is the
    /// session worktree (production == the host's session cwd — registered
    /// simplification), and the environment digest covers the harness's
    /// explicit `TestRunner::env` entries (the fixed platform allowlist is
    /// host-process-stable and excluded — registered). The ticket wraps the
    /// call BEFORE ToolStarted (same ordering discipline as file_write).
    pub(crate) async fn acaf_command_exec_event(
        &self,
        writer: &mut EventWriter<'_>,
        tool: &str,
        runner: &crate::host::TestRunner,
        activation_id: Option<String>,
    ) -> Result<TicketGate, AgentLoopError> {
        if self.acaf.is_none() {
            return Ok(TicketGate::Proceed);
        }
        let Some(store) = &self.snapshot_store else {
            // D-15 (2026-08-13): command cwd binds the worktree — fail-closed
            // refuses; shadow keeps the registered silent skip.
            let now = chrono::Utc::now();
            return self
                .fail_closed_refusal(
                    writer,
                    TicketKind::CommandExecV1,
                    orz_assurance::acaf::RejectCode::MissingSnapshotStore,
                    "no snapshot store configured — command cwd cannot be bound".to_string(),
                    &now,
                )
                .await;
        };
        if runner.command.is_empty() {
            // D-14 (2026-08-13): empty host command → hard refusal in
            // fail-closed; shadow stays silent (the tool would fail on its
            // own, and the host should never present an empty runner).
            let now = chrono::Utc::now();
            return self
                .fail_closed_refusal(
                    writer,
                    TicketKind::CommandExecV1,
                    orz_assurance::acaf::RejectCode::MissingTargetArgument,
                    "run_tests command is empty".to_string(),
                    &now,
                )
                .await;
        }
        let cwd = store.worktree().to_string_lossy().into_owned();
        let argv = runner.command.clone();
        let env_sha = crate::acaf::command_env_sha256(&runner.env);
        let canonical_args = crate::acaf::command_exec_canonical_args(tool, &argv, &cwd, &env_sha);
        let target_digest = crate::acaf::command_exec_target_digest(&argv, &cwd, &env_sha);
        let tool_owned = tool.to_string();
        self.run_action_ticket(
            writer,
            TicketKind::CommandExecV1,
            canonical_args,
            target_digest,
            activation_id,
            move || {
                let canonical =
                    crate::acaf::command_exec_canonical_args(&tool_owned, &argv, &cwd, &env_sha);
                let digest = crate::acaf::command_exec_target_digest(&argv, &cwd, &env_sha);
                Ok((canonical, digest))
            },
        )
        .await
    }

    /// Shared Slice-2 action-ticket lifecycle for the network / command
    /// branches: sign → journal issued → RECOMPUTE the canonical arguments
    /// and target digest from the issue-time inputs (never the ticket's
    /// face values) → verify_and_consume → journal consumed|rejected.
    /// Review D1-1 (2026-08-12): for network/command the recompute is
    /// deterministic — these targets are pure string/argv objects, so this
    /// is consistency checking against the captured original input, not the
    /// external-state TOCTOU detection of file_write's FS re-read. Shadow
    /// mode: every failure path journals `control_ticket_rejected` and the
    /// tool proceeds; unticketed paths (no client / no goal yet) journal
    /// nothing.
    async fn run_action_ticket(
        &self,
        writer: &mut EventWriter<'_>,
        kind: TicketKind,
        canonical_args: serde_json::Value,
        target_digest: String,
        activation_id: Option<String>,
        live: impl Fn() -> Result<(serde_json::Value, String), String>,
    ) -> Result<TicketGate, AgentLoopError> {
        let canonical_digest = crate::acaf::canonical_arguments_digest(kind, &canonical_args);
        let Some((goal_digest, goal_version)) = self.goal_binding_snapshot() else {
            // D-15 (2026-08-13): no goal context → fail-closed refuses
            // (`missing_goal_context`); shadow keeps the silent skip.
            if self.acaf_fail_closed {
                let now = chrono::Utc::now();
                return self
                    .fail_closed_refusal(
                        writer,
                        kind,
                        orz_assurance::acaf::RejectCode::MissingGoalContext,
                        "goal digest not pinned".to_string(),
                        &now,
                    )
                    .await;
            }
            return Ok(TicketGate::Proceed);
        };
        let session_id = self
            .session_id
            .clone()
            .unwrap_or_else(|| writer.run_id().to_string());
        let now = chrono::Utc::now();
        // Sign (lock scope 1 — review P2-6 2026-08-12: the mutex covers only
        // the signer RPCs, NOT the live re-derivation below).
        let signed = {
            let Some(acaf) = &self.acaf else {
                return Ok(TicketGate::Proceed);
            };
            let mut client = acaf.lock().await;
            match client
                .ensure_initialized(
                    &session_id,
                    "main",
                    goal_version,
                    &goal_digest,
                    self.policy_revision(),
                )
                .await
            {
                Ok(()) => match client
                    .sign_ticket(
                        kind,
                        activation_id.clone(),
                        &canonical_digest,
                        Some(target_digest.clone()),
                        None,
                    )
                    .await
                {
                    Ok(t) => Ok(t),
                    Err(e) => Err(crate::acaf::TicketOutcome::SignerUnreachable {
                        kind,
                        detail: e.to_string(),
                    }),
                },
                Err(e) => Err(crate::acaf::TicketOutcome::SignerUnreachable {
                    kind,
                    detail: e.to_string(),
                }),
            }
        };
        let ticket = match signed {
            Ok(t) => t,
            Err(outcome) => {
                self.journal_ticket_outcome(writer, &outcome, &now).await?;
                return match crate::acaf::ticket_outcome_reject(&outcome) {
                    None => Ok(TicketGate::Proceed),
                    Some((code, detail)) => Ok(if self.acaf_fail_closed {
                        TicketGate::Blocked { code, detail }
                    } else {
                        TicketGate::Proceed
                    }),
                };
            }
        };
        writer
            .record(
                EventType::ControlTicketIssued,
                crate::acaf::issued_payload(&ticket),
            )
            .await?;
        // Verify: recompute the canonical arguments and target digest from
        // the ORIGINAL model argument / host-owned command — never from the
        // ticket's face values. Any divergence between the two computations
        // hits `target_mismatch` (check 5/5b). Review D1-1 (2026-08-12):
        // for network/command this is a deterministic recompute of captured
        // inputs (consistency check); file_write is the true FS re-read.
        let live_outcome = match live() {
            Ok((live_args, live_target_digest)) => {
                let live_canonical_digest =
                    crate::acaf::canonical_arguments_digest(kind, &live_args);
                let Some(acaf) = &self.acaf else {
                    return Ok(TicketGate::Proceed);
                };
                let mut client = acaf.lock().await;
                client
                    .verify_and_consume(
                        &ticket,
                        &live_canonical_digest,
                        activation_id.clone(),
                        Some(live_target_digest),
                        None,
                    )
                    .await
            }
            Err(e) => Ok(crate::acaf::TicketOutcome::Rejected {
                ticket_id: Some(ticket.ticket_id.clone()),
                kind,
                code: orz_assurance::acaf::RejectCode::TargetMismatch,
                detail: format!("live target resolution failed: {e}"),
            }),
        };
        let outcome = match live_outcome {
            Ok(outcome) => outcome,
            // Review fix (2026-08-13): do NOT journal inside the arm — the
            // shared journal below emits exactly ONE terminal event per
            // ticket (the fail-closed slice originally journaled this
            // path twice).
            Err(e) => crate::acaf::TicketOutcome::SignerUnreachable {
                kind,
                detail: e.to_string(),
            },
        };
        self.journal_ticket_outcome(writer, &outcome, &now).await?;
        match crate::acaf::ticket_outcome_reject(&outcome) {
            None => Ok(TicketGate::Proceed),
            Some((code, detail)) => Ok(if self.acaf_fail_closed {
                TicketGate::Blocked { code, detail }
            } else {
                TicketGate::Proceed
            }),
        }
    }
}
