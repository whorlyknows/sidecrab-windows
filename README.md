# 🦀 Sidecrab (Windows Native Adaptation)

> **Ultra-optimized desktop pet for Claude Code on Windows.**  
> Written in pure Rust using native Win32 APIs (`WS_EX_LAYERED`).  
> **Zero Chromium. Zero Electron. Zero WebView2.**  
> **Strict <10MB RAM ceiling** (measured peak **~4.95 MB**, average **~1.83 MB**, idle CPU **~0.005%**).

---

## ⚡ Key Highlights & Architecture

- **True Native Windows Rendering**:
  - Borderless, topmost layered window (`WS_POPUP | WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW`).
  - Pre-multiplied 32-bit ARGB top-down DIB framebuffer updated via Win32 `UpdateLayeredWindow`.
  - Zero-latency per-pixel click-through: `WM_NCHITTEST` returns `HTTRANSPARENT` for $\alpha=0$ pixels, allowing background clicks to pass directly to underlying applications.
- **Strict Ultra-Low Footprint**:
  - **Memory Working Set**: Peak **4.95 MB** during active animation transitions; idle average **~1.83 MB** (passing the 10MB budget by >50%).
  - **CPU Utilization**: **0.005%** average idle CPU on modern Windows hardware.
  - **Binary Size**: Self-contained release binaries (`sidecrab-pet.exe`: 339 KB; `sidecrab-hook.exe`: 249 KB) with no external runtimes or DLL requirements.
- **Full Mascot & Animation Parity + Tester Menu**:
  - **Animations**: Neutral stand, micro-idles (blink, stretch, peek, shuffle), thinking (seated pondering with dynamic thought bubble), working (typing at laptop), permission alert (energetic claw wave), done (celebration hop), sleep/nap, panic/carried pose, glare, chase, and border wandering strolls.
  - **🎭 Animation Tester Submenu**: Right-click the crab -> **🎭 Test Animations** to test all 17 mascot animations on demand, plus **Resume Live State** to return to real-time Claude Code tracking.
  - **Hats**: Top hat, chef's hat, fedora, and animated helicopter hat with synchronized dual-phase spinning rotor.
  - **Scaling**: Pixel-crisp nearest-neighbor integer scaling (Small: 102x96, Medium: 153x144, Large: 204x192) preserving the 51:48 aspect ratio.
- **Physics & Easter Eggs**:
  - **Fling & Momentum Toss**: Drag and throw the crab with your mouse; it carries velocity with air friction and elastically bounces off monitor work area edges.
  - **🚀 Fly & Bounce (Helicopter Flight Mode)**: Right-click -> **🚀 Fly & Bounce** equips the helicopter hat and flies across the screen in sinusoidal floating arcs, bouncing off edges before smoothly gliding home and landing.
  - **🐾 Smart Inactivity Wandering**: Automatically detects user absence via Win32 `GetLastInputInfo` (>=20s inactivity) and takes walking strolls; immediately scurries home when the user touches mouse or keyboard. Trigger on demand via **🐾 Take a Walk Now**.
- **Robust Shell Interactions & State Watchdog**:
  - **Non-blocking Dragging**: Smooth `SetCapture` dragging with cute panic/carried pose; never conflicts with message loops or causes animation lock.
  - **15-Second Stale State Watchdog**: Automatically resets unclosed tool or thinking states back to `idle` if Claude Code has been silent for 15s.
  - **🔄 Reset State to Idle**: Manual instant recovery option in context menu.
  - **Double-click**: Focuses and brings the active Claude Code session host window (Windows Terminal, VS Code, PowerShell, etc.) to the foreground using Win32 `AttachThreadInput` + `SetForegroundWindow`.
  - **Right-click Context Menu**: Native Win32 menu for Size, Corner Presets, Hats, Test Animations, Fly & Bounce, Wander Now, Reset, Autostart at login, Install/Remove hooks, and Exit.
- **Claude Code Hooks CLI (`sidecrab-hook.exe`)**:
  - Handles all 8 lifecycle events: `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Notification`, `PermissionRequest`, `Stop`, `SessionStart`, `SessionEnd`.
  - Hardened Windows pipe reader (`PeekNamedPipe`) for non-blocking stdin ingestion.
  - Atomic state writes to `%USERPROFILE%/.sidecrab/state.json` and multi-session tracking under `sessions.d/`.
  - Idempotent installer/uninstaller for `%USERPROFILE%/.claude/settings.json` with automatic `.bak` backup.

---

## 📊 Benchmark Verification

Automated 30+ second stress verification with continuous state transitions (`tests/run_all_tests.ps1 -IncludeBenchmark`):

| Metric | Target / Ceiling | Measured Result | Status |
|---|---|---|---|
| **Peak Working Set Memory** | `< 10.0 MB` | **4.949 MB** | ✅ **PASS** |
| **Average Working Set Memory** | `< 5.0 MB` | **1.828 MB** | ✅ **PASS** |
| **Idle CPU Usage** | `<= 0.20%` | **0.005%** | ✅ **PASS** |
| **Browser Runtime Instances** | `0` | **0** (No Chromium/WebView2/Electron) | ✅ **PASS** |
| **Binary Sizes** | `< 1 MB` each | `sidecrab-pet`: 339 KB, `sidecrab-hook`: 249 KB | ✅ **PASS** |
| **Automated E2E Suites (Tiers 1-4)** | 100% Pass | **7/7 suites passed (0 failures)** | ✅ **PASS** |

---

## 🚀 Installation & Usage

you could also just install the exe file at realeases

### 1. Build Binaries
```powershell
cargo build --release
```
Binaries will be placed in `target\release\`:
- `target\release\sidecrab-pet.exe` (the desktop mascot application)
- `target\release\sidecrab-hook.exe` (the Claude Code hook CLI)

### 2. Run the Pet
```powershell
.\target\release\sidecrab-pet.exe
```
The crab appears resting in the bottom-right corner of your desktop.

### 3. Install Claude Code Hooks
Right-click the crab and select **Install Hooks**, or run via CLI:
```powershell
.\target\release\sidecrab-hook.exe install
```
This safely updates `%USERPROFILE%\.claude\settings.json` and creates a backup at `settings.json.bak`.

To uninstall hooks at any time:
```powershell
.\target\release\sidecrab-hook.exe uninstall
```

---

## 🧪 Running the Verification Test Suite

Run the full automated test suite including memory and CPU benchmarks:
```powershell
powershell -ExecutionPolicy Bypass -File tests\run_all_tests.ps1 -IncludeBenchmark
```

To run individual tiers:
```powershell
# Tier 1: Feature Coverage
powershell -ExecutionPolicy Bypass -File tests\tier1_feature_coverage.ps1

# 30-second RAM memory profiler
powershell -ExecutionPolicy Bypass -File tests\ram_benchmark.ps1
```

---

## 📁 Project Structure

```
claude pet/
├── Cargo.toml                  # Workspace configuration (opt-level = "z", LTO, panic = "abort")
├── crates/
│   ├── common/                 # Protocol definitions, palette, state structures
│   ├── hook/                   # sidecrab-hook CLI, settings.json manager, host detection
│   └── pet/                    # Win32 layered window, 32-bit ARGB DIB, anim state machine, hats
├── tests/                      # 4-tier E2E verification suite & benchmark harnesses
│   ├── run_all_tests.ps1       # Master test runner
│   ├── ram_benchmark.ps1       # 30s RAM profiler (<10MB ceiling assertion)
│   ├── cpu_benchmark.ps1       # Idle CPU profiler (<=0.2% assertion)
│   └── hook_sim.ps1            # Claude Code hook simulation suite
└── target/release/             # Compiled release binaries (<350 KB each)
```
