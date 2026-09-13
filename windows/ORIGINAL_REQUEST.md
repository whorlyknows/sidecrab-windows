# Original User Request

## 2026-09-12T03:23:30Z

A native, ultra-optimized Windows desktop pet adaptation of Sidecrab (https://github.com/zvoque/sidecrab) for Claude Code written in pure Rust using native Win32 APIs, operating with zero Chromium/Electron/WebView2 processes and strictly under 10MB RAM at all times.

Working directory: a:\CODE\claude pet
Integrity mode: development

## Requirements

### R1. Native Win32 Desktop Pet Window & Ultra-Low Memory Budget (<10MB RAM)
The application must run as a standalone native Windows process with zero browser engine dependencies (no Chromium, Electron, WebView2, or CEF). The pet window must be an always-on-top, borderless layered window (`WS_EX_LAYERED`, `WS_EX_TOPMOST`, `WS_EX_TOOLWINDOW`) utilizing 32-bit ARGB pixel buffers (`UpdateLayeredWindow`). Memory consumption must never exceed 10MB working set at any time during execution (typical budget 2-5MB). Window must support per-pixel click-through: clicking transparent pixels passes input directly to underlying windows (`HTTRANSPARENT`), while clicking opaque pixels allows dragging and interactions.

### R2. Complete Mascot Animations, Hats, and Interaction Parity
The mascot must replicate all visual states and interactions from Sidecrab:
- **Animations**: Idle resting, micro-idles (blink, stretch, peek, shuffle), thinking (pondering pose with thought bubble), working (seated typing at laptop), awaiting permission (energetic double-claw waving alert), done celebration, and idle wandering strolls around screen borders.
- **Hats System**: Top hat, chef's hat, fedora, and animated helicopter hat (with spinning rotor), rendered dynamically aligned to the crab's head across all animations.
- **Interactions**: Drag anywhere on screen to reposition (persisting new home coordinates), double-click to bring the Claude Code host application/terminal to the foreground, and right-click native Win32 context menu with options for:
  - Sizes: Small (102x96), Medium (153x144), Large (204x192) preserving 51:48 aspect ratio
  - Corner Presets: Top-Left, Top-Right, Bottom-Left, Bottom-Right, and Reset
  - Hats: None, Top hat, Chef, Fedora, Helicopter
  - Wander Mode toggle
  - Launch at login (Windows Run registry key or Startup shortcut)
  - Install / Remove Claude Code hooks
  - Exit

### R3. Windows Claude Code Hook Handler & Settings Integration
Provide a standalone, ultra-fast hook CLI (`sidecrab-hook.exe`) invoked by Claude Code hooks (`UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Notification`, `PermissionRequest`, `Stop`, `SessionStart`, `SessionEnd`).
- Reads JSON payloads from stdin and maintains session state in `%USERPROFILE%/.sidecrab/state.json` and session tracking files in `%USERPROFILE%/.sidecrab/sessions.d/`.
- Includes an idempotent hook installer and remover that manages `%USERPROFILE%/.claude/settings.json`, safely backing up any existing file to `settings.json.bak` before modifying, and cleanly adding or pruning entries marked with `sidecrab-hook`.

### R4. Automated Verification & Benchmarking Suite
Provide programmatic test and verification scripts:
- Automated RAM measurement script that samples `WorkingSet64` and `PrivateMemorySize64` of the running process over 30+ seconds of continuous animation and state changes, failing if memory ever exceeds 10MB.
- Hook simulation test suite that feeds mock Claude Code stdin events into `sidecrab-hook.exe` and verifies state transitions, label updates, and clean reset on session end.

## Acceptance Criteria

### Performance & Footprint
- [ ] Process working set memory (RSS) remains strictly <= 10MB under all operational states (idle, typing, wandering, menu open), verified via `Get-Process`.
- [ ] No `msedgewebview2.exe`, `electron.exe`, node, or Chromium processes are spawned.
- [ ] Idle CPU usage is <= 0.2% on standard Windows hardware.

### Functionality & Fidelity
- [ ] Native Win32 layered window renders with transparent background and crisp pixel art across S, M, and L sizes.
- [ ] Clicks on transparent window areas pass through immediately to background desktop windows without latency.
- [ ] Dragging the mascot moves the window smoothly and persists the new coordinate as home.
- [ ] Double-clicking focuses the terminal or editor hosting the Claude Code session.
- [ ] Right-click displays a native Win32 context menu with functioning Size, Position, Hat, Wander, Autostart, Hook, and Exit actions.
- [ ] Claude Code hook events (`prompt`, `pre`, `post`, `notify`, `permreq`, `stop`) update the crab's visual state and mood in real time.
- [ ] Hook installation into `%USERPROFILE%/.claude/settings.json` is atomic, creates a pristine backup, and uninstalls cleanly.
- [ ] `cargo build --release` succeeds producing self-contained release binaries.
