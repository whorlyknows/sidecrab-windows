# Project: Claude Code Desktop Pet (Sidecrab Native Win32 Port)

## Architecture
A native, ultra-optimized Windows desktop pet for Claude Code written in pure Rust using native Win32 APIs (`windows-sys`), operating with zero Chromium/Electron/WebView2 processes and strictly under 10MB RAM at all times (typical operational RSS 2-4MB).

```
+---------------------------------------------------------------------------------+
|                               WINDOWS DESKTOP                                   |
|                                                                                 |
|  [Claude Code CLI / IDE]                         [sidecrab-pet.exe]             |
|          |                                                |                     |
|    executes hooks                                    Win32 Msg Loop             |
|          v                                                |                     |
|  [sidecrab-hook.exe]                                      v                     |
|          |                             +-------------------------------------+  |
|          | write atomic tmp/rename     | Window: WS_POPUP, WS_EX_LAYERED,    |  |
|          v                             |         WS_EX_TOPMOST, TOOLWINDOW   |  |
|  %USERPROFILE%/.sidecrab/              |                                     |  |
|     ├── state.json  <==================| IPC: ReadDirectoryChangesW / Timer  |  |
|     └── sessions.d/                    | Framebuffer: 32-bit ARGB DIB        |  |
|                                        | Blit: UpdateLayeredWindow(ULW_ALPHA)|  |
|                                        | Click-Through: WM_NCHITTEST         |  |
|                                        +-------------------------------------+  |
+---------------------------------------------------------------------------------+
```

### Key Subsystems
1. **Windowing & Layered Renderer**: Borderless, topmost, taskbar-hidden layered window using 32-bit ARGB top-down DIB section, pre-multiplied alpha blitting via `UpdateLayeredWindow`, and per-pixel hit-testing (`WM_NCHITTEST`) returning `HTTRANSPARENT` for $\alpha=0$ and `HTCLIENT` for mascot body.
2. **Sprite Engine & Animation Controller**: Palette-indexed frame bitmaps in `.rodata` (<55KB), integer nearest-neighbor scaling (S: 102x96, M: 153x144, L: 204x192), 21 mascot animation states (idle, micro-idles, thinking with thought bubble, typing at laptop, claw wave alert, celebrate, walk/wander, sleep), dynamic hats overlay with precomputed head anchors and animated helicopter rotor.
3. **Shell Interaction Engine**: Borderless modal dragging with coordinate persistence in `%USERPROFILE%/.sidecrab/config.json`, double-click host activation (`AttachThreadInput` + `SetForegroundWindow`), native Win32 context menu (`TrackPopupMenuEx`), taskbar-aware corner positioning (`SPI_GETWORKAREA`), screen border wander physics, and autostart registry management (`HKCU\...\Run`).
4. **Claude Code Hook CLI & Settings Manager**: Standalone `sidecrab-hook.exe` supporting all 8 Claude Code lifecycle events, atomic JSON state writer, multi-session tracking in `sessions.d/`, host process detection, and idempotent `%USERPROFILE%/.claude/settings.json` installer/remover with pristine `.bak` backup.
5. **Automated Verification & Benchmarking Suite**: Automated RAM benchmark sampling `WorkingSet64` and `PrivateMemorySize64` over 30+ seconds under continuous state transitions (<10MB limit), CPU benchmark (<=0.2% idle), hook simulation suite, and 4-tier E2E testing suite.

---

## Feature Inventory

| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Borderless Layered Window | Win32 window with `WS_POPUP`, `WS_EX_LAYERED`, `WS_EX_TOPMOST`, `WS_EX_TOOLWINDOW` | M1 | ORIGINAL_REQUEST §R1 |
| 2 | 32-bit ARGB Framebuffer | Top-down DIB section with pre-multiplied alpha compositing via `UpdateLayeredWindow` | M1 | ORIGINAL_REQUEST §R1 |
| 3 | Per-Pixel Click-Through | `WM_NCHITTEST` returning `HTTRANSPARENT` for alpha==0, `HTCLIENT` for opaque pixels | M1 | ORIGINAL_REQUEST §R1 |
| 4 | Low-Power Message Loop | Kernel-wait message loop (`GetMessageW` / `MsgWaitForMultipleObjectsEx`) achieving <=0.2% idle CPU | M1 | ORIGINAL_REQUEST §R1 |
| 5 | Strict <10MB Working Set | Zero-waste allocation, embedded `.rodata` assets, process working set strictly <10MB (target 2-4MB) | M1 | ORIGINAL_REQUEST §R1 |
| 6 | Integer Nearest-Neighbor Scaling | Scaled pixel art for Small (102x96, 2x), Medium (153x144, 3x), Large (204x192, 4x) | M2 | ORIGINAL_REQUEST §R2 |
| 7 | Core Mascot Animations | Resting idle, ambient blink, micro-idles (blink, stretch, peek, shuffle, look, wave) | M2 | ORIGINAL_REQUEST §R2 |
| 8 | Claude Mood Animations | Thinking (pose + dynamic typing thought bubble), Working (seated typing at laptop), Alert (claw wave) | M2 | ORIGINAL_REQUEST §R2 |
| 9 | Lifecycle Animations | Celebrate (double hop decay), Sleep (narcolepsy after idle), Wake-up | M2 | ORIGINAL_REQUEST §R2 |
| 10 | Hats System | Top hat, Chef's hat, Fedora with precomputed frame anchor alignment | M2 | ORIGINAL_REQUEST §R2 |
| 11 | Animated Helicopter Hat | Helicopter hat with dual alternating spinning rotor frames synchronized to ticks | M2 | ORIGINAL_REQUEST §R2 |
| 12 | Border Wander Mechanics | Walk cycle (frames 5-19), screen border strolls, boundary collision, homing to base position | M2 | ORIGINAL_REQUEST §R2 |
| 13 | Smooth Window Dragging | Borderless window drag (`WM_NCLBUTTONDOWN` with `HTCAPTION`), persisting home coordinates | M3 | ORIGINAL_REQUEST §R2 |
| 14 | Double-Click Focus Host | Double-click mascot brings Claude Code terminal / editor host window to foreground | M3 | ORIGINAL_REQUEST §R2 |
| 15 | Native Win32 Context Menu | Right-click popup menu with Size, Corner Presets, Hats, Wander, Autostart, Hook, Exit | M3 | ORIGINAL_REQUEST §R2 |
| 16 | Corner Presets & Placement | Instant positioning: Top-Left, Top-Right, Bottom-Left, Bottom-Right, Reset (work area aware) | M3 | ORIGINAL_REQUEST §R2 |
| 17 | Autostart Integration | Toggle launch at login via Windows Registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) | M3 | ORIGINAL_REQUEST §R2 |
| 18 | Standalone Hook CLI | Standalone `sidecrab-hook.exe` invoked by Claude Code lifecycle events | M4 | ORIGINAL_REQUEST §R3 |
| 19 | Hook Event Ingestion | Handles UserPromptSubmit, PreToolUse, PostToolUse, Notification, PermissionRequest, Stop, SessionStart, SessionEnd | M4 | ORIGINAL_REQUEST §R3 |
| 20 | State Persistence & IPC | Atomic writes to `%USERPROFILE%/.sidecrab/state.json` and multi-session `sessions.d/` | M4 | ORIGINAL_REQUEST §R3 |
| 21 | Host Process Detection | Detects host terminal/IDE PID/HWND via parent process traversal and records in state.json | M4 | ORIGINAL_REQUEST §R3 |
| 22 | Idempotent Settings Hook Installer | Atomically installs hook configuration into `%USERPROFILE%/.claude/settings.json` with `.bak` backup | M4 | ORIGINAL_REQUEST §R3 |
| 23 | Surgical Hook Uninstaller | Cleanly removes only `sidecrab-hook` entries from `settings.json`, preserving other tools and user config | M4 | ORIGINAL_REQUEST §R3 |
| 24 | E2E Opaque-Box Test Suite | Tiers 1-4 test suite validating features, boundaries, combinations, and real-world workflows | E2E-TEST | ORIGINAL_REQUEST §R4 |
| 25 | Automated RAM Benchmark | 30+ second benchmark sampling `WorkingSet64` and `PrivateMemorySize64` under state changes (<10MB limit) | M5 | ORIGINAL_REQUEST §R4 |
| 26 | CPU Idle Benchmark | Verification that idle CPU consumption is <= 0.2% | M5 | ORIGINAL_REQUEST §R4 |
| 27 | Hook Simulation Test Suite | Feeds mock Claude Code stdin events into `sidecrab-hook.exe` and verifies state transitions and reset | M5 | ORIGINAL_REQUEST §R4 |
| 28 | Adversarial Coverage Hardening | Tier 5 white-box stress testing and edge-case hardening | M5 | ORIGINAL_REQUEST §R4 |
| 29 | Self-Contained Release Build | `cargo build --release` producing zero-dependency release binaries | M5 | ORIGINAL_REQUEST Acceptance |

---

## Milestones

| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| E2E | E2E Testing Suite | Requirements-driven test infra, test cases (Tiers 1-4), publishes `TEST_READY.md` | none | PLANNED |
| M1 | Core Pet Window & Layered Render Engine | Window creation (`WS_EX_LAYERED`), 32-bit ARGB DIB, `UpdateLayeredWindow`, `WM_NCHITTEST` click-through, message loop, low CPU/RAM baseline | none | PLANNED |
| M2 | Sprite Engine, Mascot Animations & Hats | Embedded `.rodata` assets, 21 animations, thought bubble/typing overlay, 5 hats with spinning rotor, wander physics | M1 | PLANNED |
| M3 | Shell Interactions & Context Menu | Drag & persist coordinates, double-click focus host, Win32 context menu, corner presets, autostart | M1, M2 | PLANNED |
| M4 | Claude Code Hook CLI & Settings Manager | `sidecrab-hook.exe`, 8 hook events, atomic state IPC, host detection, idempotent settings.json install/uninstall with `.bak` | none | PLANNED |
| M5 | Final E2E Integration, Benchmarking & Hardening | Phase 1: 100% pass of E2E test suite (Tiers 1-4); Phase 2: 30s RAM benchmark (<10MB), CPU benchmark (<=0.2%), Tier 5 adversarial hardening | E2E, M1, M2, M3, M4 | PLANNED |

---

## Interface Contracts

### 1. Window Renderer (`render`) ↔ Animation Engine (`anim`)
- **Canvas Dimensions**: Base logical canvas is 51x48 pixels. Mascot origin is at `y = 12`.
- **Scaling**: Target sizes Small (102x96, scale=2), Medium (153x144, scale=3), Large (204x192, scale=4).
- **Framebuffer Blit**:
  ```rust
  pub fn blit_frame(
      dst_bits: *mut u8,
      dst_width: i32,
      dst_height: i32,
      scale: u32,
      frame_idx: usize,
      hat: HatType,
      hat_frame: usize,
      overlay: Option<&OverlayState>,
  );
  ```
- **Hit-Testing**:
  ```rust
  pub fn is_pixel_opaque(dst_bits: *const u8, width: i32, height: i32, x: i32, y: i32) -> bool;
  ```

### 2. State & IPC (`state`) ↔ Desktop Pet Window (`app`)
- **File Path**: `%USERPROFILE%/.sidecrab/state.json`
- **Schema**:
  ```json
  {
    "version": 1,
    "state": "idle" | "thinking" | "working" | "awaiting_permission" | "celebrate" | "alert" | "sleep",
    "mood": "neutral" | "happy" | "focused" | "excited" | "curious" | "alert",
    "tool": null | string,
    "prompt_preview": null | string,
    "active_session_id": string,
    "timestamp": number,
    "host_hwnd": number,
    "host_pid": number
  }
  ```
- **Notification**: `ReadDirectoryChangesW` on `%USERPROFILE%/.sidecrab/` posts `WM_USER + 101` (`WM_SIDECRAB_STATE_CHANGED`) to `pet_hwnd`, with fallback timer poll (500ms).

### 3. Settings Manager (`settings`) ↔ Claude Code (`claude`)
- **Path**: `%USERPROFILE%/.claude/settings.json`
- **Backup**: `%USERPROFILE%/.claude/settings.json.bak` (created only if backup does not exist or original has changed).
- **Hook Entries**:
  ```json
  {
    "hooks": {
      "UserPromptSubmit": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["UserPromptSubmit"] }],
      "PreToolUse": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["PreToolUse"] }],
      "PostToolUse": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["PostToolUse"] }],
      "PermissionRequest": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["PermissionRequest"] }],
      "Notification": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["Notification"] }],
      "Stop": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["Stop"] }],
      "SessionStart": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["SessionStart"] }],
      "SessionEnd": [{ "type": "command", "command": "<path>\\sidecrab-hook.exe", "args": ["SessionEnd"] }]
    }
  }
  ```

---

## Code Layout

```
a:/CODE/claude pet/
├── Cargo.toml                  # Workspace definition (crates: pet, hook, common)
├── Cargo.lock
├── crates/
│   ├── common/                 # Shared types, state schemas, paths, palette
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs        # StateJson, SessionInfo serialization
│   │       ├── paths.rs        # %USERPROFILE%/.sidecrab paths resolution
│   │       └── protocol.rs     # HookEvent enum and mappings
│   ├── pet/                    # Main native desktop pet application (sidecrab-pet.exe)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # WinMain entry, single instance check, message loop
│   │       ├── window.rs       # Layered window creation, WndProc, HTTRANSPARENT hit test
│   │       ├── dib.rs          # 32-bit ARGB DIB section & UpdateLayeredWindow
│   │       ├── assets.rs       # Static .rodata sprite tables (crab frames, hats, bubbles)
│   │       ├── anim.rs         # Animation state machine, tick timer, micro-idles
│   │       ├── wander.rs       # Border strolls physics & homing logic
│   │       ├── menu.rs         # Native Win32 context menu (sizes, hats, corners, etc.)
│   │       ├── drag.rs         # Modal borderless dragging & coordinate persistence
│   │       ├── focus.rs        # Double click host terminal / IDE activation
│   │       ├── ipc.rs          # ReadDirectoryChangesW state watcher
│   │       └── autostart.rs    # Windows Registry HKCU\...\Run manager
│   └── hook/                   # Claude Code hook CLI (sidecrab-hook.exe)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs         # CLI entry point, stdin JSON parser, event dispatch
│           ├── host.rs         # Toolhelp32Snapshot host PID/HWND discovery
│           ├── installer.rs    # Idempotent settings.json installer & uninstaller
│           └── session.rs      # atomic state.json writer & sessions.d updater
├── tests/                      # E2E test harness and simulation tests
│   ├── e2e_suite.rs
│   ├── hook_sim.rs
│   └── ram_benchmark.ps1
└── ORIGINAL_REQUEST.md
```
