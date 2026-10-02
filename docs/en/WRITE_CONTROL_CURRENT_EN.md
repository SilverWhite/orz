# Write Control — Current Design (English distillation)

> LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.
> This is a **distillation of the current state**, not a full translation: history blocks are
> compressed to one line per version.
> Sources: [`WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)
> (**v4.0**, current rule surface authority) + [`WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)
> (**v1.0**, architecture parts declared unchanged by v4). Both are Chinese.
> State: v4.0 landed in carrier 0.8.8; the three-task re-verification passed (0ch closed 2026-10-02); in service through 0.8.10.
> Terminology: [TRANSLATION_GLOSSARY.md](TRANSLATION_GLOSSARY.md).

## 0. One-paragraph summary

Write control is a **pattern-based catastrophic hard boundary**, not a general write reviewer.
It intercepts only **irreversible destruction shapes** — root-level recursive deletion, raw
device / volume destructive writes, boot-firmware and security-mechanism flips, registry hive
deletion, and **host-state writes** (the `.gsa` session volume and the ACAF keystore root /
signer manifest, plus sweep-style deletions of their ancestors). Everything else — installing
into `/usr`, clearing `C:\` caches, deleting stale patches, editing `/etc`, `>/dev/null` — is
allowed and handed to the approval component (the Codex-lineage permission chain). The carrier
itself (install directory, the three binaries, `grok-home`) is **deliberately not protected**:
recoverable, disposable state goes to the approval component and recovery mechanisms, not to a
hard deny.

Positioning is a **backstop** with three grades of wording — *guarantee / resistance / audit* —
never an absolute guarantee. There is no writable-root allowlist.

## 1. The four tiers (L0–L4, v1 architecture unchanged by v4)

| Tier | What it does | State |
|---|---|---|
| L0 normalization | Write targets and in-command paths go through canonical/lexical normalization before matching (device-prefix stripping, near-ancestor canonicalization, ASCII case folding on Windows, `..` folding). Fail-closed: table resolution failures fall back to literal defaults, never to allow. | coded |
| L1 deny half | Single-source deny table shared by all consumption points | narrowed to the carrier set (see §2) |
| L2 command review | `run_terminal_cmd` rule library: block / warn / allow, best-effort lexical matching, every decision logged with the journal | coded, 5-rule closed enumeration |
| L3 process-level enforcement | Linux = Landlock (spawn-time allow-set, disaster exclusions); Windows = CFA probe path | coded on Linux; kernel-granularity precision landed 0.8.8 |
| L4 compensation | Edit-face rollback window (pre-write byte snapshots, 5 per file), `orz rollback list/restore` undo CLI, carrier integrity self-check (manifest-based, two-state liveness probe) | partially coded |

Design invariants: the deny table is **compiled-in constants** — no runtime config file, no
escape hatch, no model-reachable off switch; changing the table means changing code (plus
tests and this document family). Table changes are pinned by line-count and inclusion-relation
assertions ("first red, then green").

## 2. The rule surface (closed enumeration; exactly 5 block rules)

L1's deny surface narrowed to the **carrier set**: C1 the `.gsa` session volume domain, and
C2′ the ACAF keystore root + signer manifest (resolved from wiring-sourced env
`ORZ_ACAF_KEYSTORE` / `ORZ_ACAF_MANIFEST`). The old install-directory / three-binaries /
`grok-home` protections are fully retired (see §4).

L2 reviews `run_terminal_cmd` against exactly five block rules (closed verb tables × closed
target shapes; no heuristics, no "looks dangerous" scoring, no broad path-prefix scans;
anything off-table is always allowed):

| # | Rule id | Trigger shape | Examples |
|---|---|---|---|
| 1 | `catastrophic-recursive-delete` | Delete verbs (`rm`/`rmdir`/`rd`/`del /s`/`Remove-Item`/`erase`/`ri`) × target = **volume root** (`/`, `C:\`, `D:\`…) or **fundamental tree root** (Windows: `C:\Windows`, `C:\Program Files`, `C:\Program Files (x86)`, `C:\ProgramData`; Linux: `/boot`, `/etc`, `/usr`, `/bin`, `/sbin`, `/lib`, `/lib32`, `/lib64`, `/var`, `/dev`, `/proc`, `/sys`) + recursive flag (`-r`/`-rf`/`-R`/`--recursive`/`/s`/`-Recurse`). Drive-relative root forms (`\Windows`, `/usr`) get the drive letter filled in from `%SystemDrive%` before matching. | `rm -rf /`; `Remove-Item C:\Windows -Recurse`; `rm -rf /usr`; `rm -rf \Windows` |
| 2 | `raw-device-write` | `dd` with `of=` landing on a **block device** (`/dev/[sv]d*`, `/dev/vd*`, `/dev/nvme*`, `/dev/mmcblk*`, `/dev/mapper*`, `\\.\PhysicalDrive*`; **`/dev/null` explicitly exempt**); `mkfs*` with a target landing on those shapes (image-file builds like `mkfs.ext4 disk.img` are allowed); `format`; `diskpart` (script form); shadow-copy deletion (`vssadmin delete` / `wbadmin delete`). Target-position discipline: `PhysicalDrive` tokens only match on the write-side target (`of=` value / `mkfs*` target) — read-side `dd if=\\.\PhysicalDrive0` (backup/forensics) is allowed. | `dd if=x of=/dev/sda`; `mkfs.ext4 /dev/sdb` |
| 3 | `boot-firmware-flip` | `bcdedit`; Defender preference domain (`Set-MpPreference` etc.); firewall profile set; `sc/net stop windefend\|mpssvc`; `Set-ExecutionPolicy`; audit log clearing (`wevtutil cl` / `Clear-EventLog`); `fltmc unload` | (full set carried over from v1 `safety-mechanism-flip`) |
| 4 | `registry-hive-delete` | `reg delete\|add\|import` targeting `HKLM`/`HKCR`/`HKU` — target-position matching on the third token; hive mentions inside data values (`/d hklm-…`) are not false-positived | (carried over from v1) |
| 5 | `carrier-write` | **The two narrow host-state targets** (① ② below) + the **ancestor-chain arm** (③): ① the `{cwd}/.gsa` session volume (container side = host bind mount; a container-side ACAF keystore inside the volume is covered too; on double coverage C1 reports first, as `carrier:session-volume`); ② the **ACAF keystore root** (keystore directory + signer manifest + the two key files; host-native location resolved from wiring-sourced env, typically `<install>\<acaf>\keystore`); ③ under delete/move verbs (`ANCESTOR_SWEEP_VERBS`, a closed subset of exactly 15), a target that is a **strict ancestor** of ① or ② also lands here — e.g. `rm -rf <install>` sweeping away a keystore that lives inside the retired install directory. Direct hits report first; placement writes (`cp`/`mkdir`/`install`) never trigger the arm; volume-root recursive deletion is caught by rule 1 first. | rule id unchanged (schema v0.3 enumeration — replay-compatible); target definition = "host state that still exists after the session ends" |

**Block-refusal copy**: the envelope carries the rule id + target + one line "you have crossed
the backstop's hard boundary"; rule 1 additionally appends the user's own guidance — "use a
precise deletion instead" (naming the blocked target and suggesting a specific file/subdirectory).

**Warn tier (logged, never blocks, not counted as interceptions)**: `elevation` (appearance of
`sudo`/`runas`); `broad-destructive`'s remaining arms (non-root recursive deletes, non-cwd wide
deletes) — pure logging.

## 3. What it deliberately does NOT do (surfaces retired and returned to the approval component)

1. **No whole-tree position locks**: v1 denied all writes into the Windows system roots and the
   Linux twelve system trees. Since v2, file/subdirectory-level create, modify, delete, and
   install inside those trees is **not mechanically blocked** (`make install`, `apt install`,
   editing `/etc/hosts`, clearing `C:\Windows\SoftwareDistribution` are all allowed). The
   "fundamental tree roots" survive only as **rule 1's recursive-delete targets**.
2. **No redirection scanning**: `>` / `>>` redirection targets are no longer scanned
   (`>/dev/null`, `2>/dev/null`, `+O/dev/null` all pass). Accepted consequence (registered):
   `echo x > /proc/sys/...` (sysctl writes) is allowed with no warning — within the "ordinary
   writes go back to the approval component" adjudication.
3. **No `/dev` / `/proc` / `/sys` prefix-token scanning** — only rule 2's `dd of=block-device` shape
   remains (reading `/proc/cpuinfo` or any mention of `/dev/null` is never blocked).
4. **No carrier self-protection**: install directory / three binaries / `grok-home` /
   install-dir-inside-cwd downgrade rules — fully retired in v3. Reasons (user adjudication):
   guard self-protection is circular (deleting files cannot kill a running guard, and future
   runs always start from a freshly deployed carrier); recoverability is the classification
   standard (the backstop only blocks the *irreversible*; the carrier is minutes-from-release-
   package recoverable); and whole-directory protection had a **structurally fatal collision**
   with legitimate installs (the build-pov-ray task hardcodes `/usr/local/bin/povray` —
   the 0.8.5 rerun recorded 7 interceptions, 4 of them this collision, run `RUN-CLI-6abbb013`).
   The keystore/manifest are the exception: they are trust anchors (destroying them invalidates
   all in-flight tickets and requires re-provisioning — non-trivially recoverable, and there is
   no legitimate model-side reason to write them), so they form the host-state backstop
   together with `.gsa`.

## 4. Division of labor with the approval component (user adjudication, final)

| | Mechanical backstop (this document) | Approval component (orz-workspace/permission, Codex lineage) |
|---|---|---|
| Governs | The five catastrophic hard boundaries (irreversible destruction shapes) | Approval semantics for ordinary write actions (allow_once / approval flow / permission modes) |
| Form | Mechanical block, the model cannot turn it off | Decided by permission mode (yolo auto / interactive approval / evaluation pass-through) |
| Relationship | The **backstop** — not a reviewer, does not do the approval component's job | The **primary reviewer** — the only decision surface for routine writes |

## 5. L3 kernel precision (0ch, v4.0; landed in carrier 0.8.8)

Principle: **L3 granularity must not be coarser than L1/L2** — what L1/L2 refuse, L3 must
refuse; ordinary safe writes that L1/L2 allow should pass L3 too; L3 may only be *stricter*
(kernel-side catastrophic coverage).

- **Device-face file-level allow — `DEVICE_SAFE_NODES`, a closed table of exactly 7**:
  `/dev/null`, `/dev/zero`, `/dev/full`, `/dev/tty`, `/dev/random`, `/dev/urandom`, `/dev/ptmx`.
  Each is granted a Landlock PATH_BENEATH **file-level** rule (`WRITE_FILE` + `TRUNCATE` on
  ABI ≥ v3). Missing nodes or symlinks ⇒ skipped ⇒ default-deny (fail-closed direction). The
  `/dev` directory tree itself stays rule-less: block-device nodes and dangerous char nodes
  (`/dev/mem`, `/dev/kmem`, `/dev/port`) remain kernel-denied for open-write; node deletion
  (`rm /dev/null`) and device-node creation (`mknod`) remain denied; any off-table existing
  node (e.g. `/dev/console`) remains denied.
- **Root make-subset grant — `ROOT_MAKE_GRANT`, exactly 5 bits**: a PATH_BENEATH rule on `/`
  itself granting `MAKE_DIR|MAKE_REG|MAKE_SOCK|MAKE_FIFO|MAKE_SYM` — so `mkdir /git` style
  top-level creations pass. Excluded on purpose: `MAKE_CHAR`/`MAKE_BLOCK` (device-node
  creation stays denied tree-wide — self-made block nodes are a bypass around rule 2's raw
  device coverage) and `WRITE_FILE`/`TRUNCATE`/`REMOVE_*`/`REFER` (overwrite/truncate/delete/
  cross-directory move stay default-denied tree-wide).
- **Landlock union-semantics hard boundary (registered, accepted)**: PATH_BENEATH has no
  depth concept, so the `/` grant applies tree-wide and rules only add. Consequences: new
  entries under `/boot` or `/dev` can be *created* (assessed inert — boot flips need to
  *overwrite existing* grub/kernel files, which stays denied; new files under `/dev` cannot be
  written); deleting a created entry stays denied. `WRITE_FILE` on `/` is **vetoed
  forever**: with union semantics it would silently allow open-write on *existing* host block
  device nodes (`/dev/sda`) — the disk-wipe backstop would fail its one job.
- **Known spawn-scope boundary**: authorization binds objects that exist at spawn time; a
  top-level directory created in the same spawn has no rules of its own, so writing *files
  into it* in that same command chain still gets EACCES — split across two spawns (the next
  command's enumeration picks it up) it works. Fixed by an e2e boundary pin; if it ever
  becomes real friction, a state-split grant scheme is the recorded candidate.
- Vetoed alternatives are archived in the source document §7.6 to prevent resurrection.

## 6. Anti-inflation discipline (why "no creep" is testable)

- **Closed enumeration**: the 5 block rules are combinations of closed verb tables × closed
  target shapes. Off-table ⇒ always allow.
- **Pins (first red, then green)**: line-count assertions on every closed table (adding a row
  must change a test); a **positive allow-set fixture** that must keep passing (one regression
  = red): `make install`, `apt-get install`, `pip install`, `echo x > /dev/null`,
  `dd if=x of=/dev/null`, `Remove-Item C:\Windows\SoftwareDistribution\Download\old`,
  `cat /proc/cpuinfo`, `install to /usr/local/bin/povray`, `rm -rf /usr/local`, symlink and
  placement writes into the keystore's neighborhood (`cp backup <acaf>/backup-store`,
  `mkdir <acaf>/newdir`), and more; a **negative set** with ≥2 hit examples per rule
  (`rm -rf /`, `dd of=/dev/sda`, `bcdedit`, `reg delete HKLM`, writing `.gsa`, writing the
  keystore root, ancestor sweeps like `rm -rf <install>`); inclusion pins
  (`DELETE_VERBS ⊆ ANCESTOR_SWEEP_VERBS ⊆ DESTRUCTIVE_VERBS`; `install`/`ln`/`cp`/`mkdir` are
  write verbs but not sweep verbs); rule-order pin (`rm -rf D:\` reports rule 1, not rule 5).
- **No lane problem**: the backstop shape never obstructs a legitimate task (no real task
  recursively deletes a volume root), so no per-benchmark switches exist; "no runtime switch;
  a table change is a code change" holds.

## 7. Revision history (one line per version)

- **v1.0** (0bw, 2026-09-26): original design — single-source deny table, whole-tree system
  locks, carrier self-protection, L2 review with block/warn/allow, L3/L4 planned.
- **v2.0/2.1** (0cb, 2026-09-29): backstop rewrite — whole-tree locks and redirection scanning
  retired, five-rule closed enumeration born; schema v0.3 with legacy replay exemptions;
  several over-broad arms narrowed (target-position matching, `ri` verb, drive-relative roots).
- **v3.0** (0cc, 2026-09-29): narrowed to pure host-machine catastrophic backstop — carrier
  self-protection fully retired, rule 5 target set = `.gsa` + keystore root.
- **v3.1** (0cc S2, 2026-09-30): ancestor-chain arm + `install`/`ln` verb completion, after a
  full three-dimension review.
- **v4.0** (0ch, 2026-10-01): L3 kernel-granularity precision (§5) — landed in 0.8.8; S4
  three-task rerun re-verification passed (first-task friction eliminated, two turnarounds),
  0ch closed 2026-10-02.

Evidence trail (Chinese): carrier rebuild and in-service records live in
[`docs/audits/`](../audits/) batches 121/122/144/147/155; the TB21 interception-tax failure
dissection that triggered this line is in the
[`TB21 full-round report`](../TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md) §8.
