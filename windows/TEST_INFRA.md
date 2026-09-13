# TEST INFRASTRUCTURE SPECIFICATION: Claude Code Desktop Pet (Sidecrab Win32)

**Version**: 1.0.0  
**Scope**: End-to-End (E2E) Opaque-Box Testing, Benchmarking & Verification Architecture  
**Author**: `test_writer_1` (Dual-Track Testing Agent)  
**Target Project**: Native Windows Desktop Pet for Claude Code (`a:\CODE\claude pet`)  

---

## 1. Test Architecture Overview

The testing infrastructure implements the **Dual Track Testing Methodology** for native Windows desktop software. Because `sidecrab-pet.exe` and `sidecrab-hook.exe` are native Win32 executables interacting directly with the Windows Desktop Window Manager (DWM), Windows Shell, the filesystem, and Claude Code, testing cannot rely solely on standard in-process Rust unit tests. Instead, verification is structured as an **Opaque-Box E2E Testing Suite** combined with automated hardware-footprint benchmarking.

```
+----------------------------------------------------------------------------------------------------+
|                                    MASTER TEST RUNNER (run_all_tests.ps1)                          |
+----------------------------------------------------------------------------------------------------+
       |                              |                              |                        |
       v                              v                              v                        v
+------------------+          +------------------+          +------------------+     +-------------------+
|      TIER 1      |          |      TIER 2      |          |      TIER 3      |     |      TIER 4       |
| Feature Coverage |          | Boundary/Corners |          | Cross-Combos     |     | Real Session Flow |
| (>=5 tests/feat) |          | (>=5 tests/feat) |          | (Pairwise matrix)|     | (Full lifecycle)  |
+------------------+          +------------------+          +------------------+     +-------------------+
       |                              |                              |                        |
       +------------------------------+------------------------------+------------------------+
                                      |
                                      v
           +------------------------------------------------------+
           |       OPAQUE-BOX VERIFICATION CHANNELS               |
           | 1. Win32 P/Invoke (Window Styles, Handles, Rects)    |
           | 2. WM_NCHITTEST Hit-Testing (Per-pixel transparency) |
           | 3. Process & Memory Metrics (Get-Process, <10MB RSS) |
           | 4. Process Tree Auditing (0 Chromium/Electron/Node)  |
           | 5. Filesystem IPC State Assertions (state.json)      |
           | 6. Session Tracking Registry (sessions.d/<sid>)      |
           | 7. Settings & Backup Integrity (settings.json[.bak]) |
           | 8. Windows Registry Inspection (HKCU\...\Run)        |
           +------------------------------------------------------+
                                      |
       +------------------------------+-------------------------------+
       |                                                              |
       v                                                              v
+------------------------------------+              +-------------------------------------+
|    RAM BENCHMARK (ram_benchmark.ps1)|              |  HOOK SIMULATION (hook_sim.ps1)     |
| 30+ sec continuous state switching |              | 8 Claude Code events via stdin      |
| Strict <10MB Working Set threshold |              | Stdin JSON schemas & exit codes     |
+------------------------------------+              +-------------------------------------+
```

---

## 2. Opaque-Box Verification Channels

Opaque-box testing interacts with the compiled artifacts strictly from the outside through observable system surfaces without instrumenting internal private code.

### Channel 1: Win32 Window Handle & Style Validation
- **Method**: Direct Win32 P/Invoke via `user32.dll` (`FindWindowExW`, `GetWindowLongPtrW`, `GetWindowRect`, `IsWindowVisible`).
- **Target Window**:
  - Class Name: `ClaudePetWindowClass`
  - Window Title: `Claude Code Pet`
- **Assertions**:
  - Standard Styles: `WS_POPUP | WS_VISIBLE` (no caption, borders, or title bar).
  - Extended Styles: `WS_EX_LAYERED (0x80000) | WS_EX_TOPMOST (0x0008) | WS_EX_TOOLWINDOW (0x0080)`.
  - Geometry: Aspect ratio strictly 51:48; dimensions Small (`102x96`), Medium (`153x144`), Large (`204x192`).
  - Taskbar Exclusion: Excluded from taskbar and Alt+Tab (`WS_EX_TOOLWINDOW`).

### Channel 2: Per-Pixel Click-Through Hit-Testing
- **Method**: Send `WM_NCHITTEST` (`0x0084`) messages to window coordinates via `SendMessageW`.
- **Assertions**:
  - Transparent region (background pixels, $\alpha = 0$): returns `HTTRANSPARENT` (`-1` / `0xFFFFFFFF`), confirming clicks pass through to windows beneath.
  - Mascot body region ($\alpha > 0$): returns `HTCLIENT` (`1`), confirming mascot captures clicks for dragging and interaction.

### Channel 3: Memory Sampling & Resource Footprint
- **Method**: `Get-Process` sampling `WorkingSet64` (physical RSS) and `PrivateMemorySize64` (committed private memory).
- **Assertions**:
  - RSS strictly $\le 10\text{ MB}$ (10,485,760 bytes) at all times across idle, thinking, typing, dragging, and menu states.
  - Target operational range: $2.0\text{ MB} - 4.5\text{ MB}$.
  - Peak memory recorded and logged to benchmark report.

### Channel 4: Process Hierarchy & Engine Isolation
- **Method**: `Get-CimInstance Win32_Process` querying child processes where `ParentProcessId = $petPid`.
- **Assertions**:
  - Exactly zero web runtime engines spawned: zero instances of `msedgewebview2.exe`, `electron.exe`, `node.exe`, `chrome.exe`, `cef.exe`.
  - Mascot operates strictly as a self-contained native binary.

### Channel 5: Filesystem State Assertions (`state.json`)
- **Method**: Real-time read and JSON schema validation of `%USERPROFILE%/.sidecrab/state.json`.
- **Assertions**:
  - Schema integrity: `state`, `mood`, `label`, `tool`, `prompt`, `sessionId`, `host_hwnd`, `host_pid`, `startedAt`, `ts`.
  - Atomic writing: Validated by absence of partial/corrupt JSON during rapid concurrent updates.
  - State enum validity: values match specification (`idle`, `thinking`, `tool`/`working`, `permission`/`alert`, `done`, `sleep`).

### Channel 6: Multi-Session Tracking Registry (`sessions.d/`)
- **Method**: Filesystem directory assertions on `%USERPROFILE%/.sidecrab/sessions.d/`.
- **Assertions**:
  - Marker file existence for active session IDs.
  - Clean removal upon `SessionEnd`.
  - Isolation: ending a secondary session does not reset state owned by an active primary session.

### Channel 7: Settings & Backup Integrity (`settings.json`)
- **Method**: File inspection of `%USERPROFILE%/.claude/settings.json` and `%USERPROFILE%/.claude/settings.json.bak`.
- **Assertions**:
  - Backup creation: Pristine `.bak` created prior to modification.
  - Backup preservation: Subsequent installations never overwrite the original `.bak`.
  - Surgical removal: Uninstallation removes only `sidecrab-hook` blocks, leaving other user hooks, tools, and configs intact.
  - Valid JSON formatting after all operations.

### Channel 8: Windows Shell & Registry Integration
- **Method**: Read Windows Registry key `HKCU:\Software\Microsoft\Windows\CurrentVersion\Run`.
- **Assertions**:
  - Toggle autostart creates value `"SidecrabPet"` with executable path.
  - Disabling autostart deletes the registry value cleanly without error.

---

## 3. Mapping of 29 Features to Test Tiers

All 29 features from `PROJECT.md` Feature Inventory are comprehensively mapped across the 4 testing tiers:

| # | Feature | Milestone | Tier 1 (Coverage) | Tier 2 (Boundaries) | Tier 3 (Combinations) | Tier 4 (Workload) |
|---|---------|-----------|-------------------|---------------------|-----------------------|-------------------|
| 1 | Borderless Layered Window | M1 | T1.01 Window styles check | T2.01 Null/invalid display | T3.01 Multi-monitor drag | T4.01 Window persistence |
| 2 | 32-bit ARGB Framebuffer | M1 | T1.02 Alpha channel test | T2.02 Min/Max resolution | T3.02 Rapid size switches | T4.02 Sustained render loop |
| 3 | Per-Pixel Click-Through | M1 | T1.03 WM_NCHITTEST opaque/trans | T2.03 Boundary pixel (x=0, y=0) | T3.03 Click-through while moving | T4.03 Interactive passthrough |
| 4 | Low-Power Message Loop | M1 | T1.04 Idle thread wait | T2.04 Zero-message starvation | T3.04 Rapid message queue blast | T4.04 Idle CPU <=0.2% |
| 5 | Strict <10MB Working Set | M1 | T1.05 Baseline memory sample | T2.05 Peak alloc during blit | T3.05 Memory during resize | T4.05 30s RAM benchmark |
| 6 | Integer Nearest-Neighbor Scale | M2 | T1.06 S, M, L dimensions | T2.06 Non-standard display DPI | T3.06 Hat alignment per scale | T4.06 Scale switch during turn |
| 7 | Core Mascot Animations | M2 | T1.07 Resting, blinks, micro-idles | T2.07 Rapid state flips | T3.07 Micro-idle during drag | T4.07 Natural idle transitions |
| 8 | Claude Mood Animations | M2 | T1.08 Think, work, alert animations | T2.08 Missing tool name | T3.08 Hook event during alert | T4.08 Claude turn progression |
| 9 | Lifecycle Animations | M2 | T1.09 Celebrate, sleep, wakeup | T2.09 Sleep interrupt timing | T3.09 Sleep while wander on | T4.09 180s inactivity sleep |
| 10 | Hats System | M2 | T1.10 Top, chef, fedora hats | T2.10 Invalid hat index | T3.10 Hat persistence across size | T4.10 Hat during animation cycles |
| 11 | Animated Helicopter Hat | M2 | T1.11 Dual rotor alternating frames| T2.11 High tick rate sync | T3.11 Heli rotor while walking | T4.11 Extended rotor spin stability|
| 12 | Border Wander Mechanics | M2 | T1.12 Walk frames, border bounds | T2.12 Screen edge collision clamp | T3.12 Wander pause on hover | T4.12 Autonomous wander & homing |
| 13 | Smooth Window Dragging | M3 | T1.13 Drag repositioning | T2.13 Extreme coordinate clamp | T3.13 Drag while thinking hook | T4.13 Drag and persist home |
| 14 | Double-Click Focus Host | M3 | T1.14 Double-click activates host | T2.14 Host process terminated | T3.14 Focus during wander | T4.14 Double-click in Claude turn |
| 15 | Native Win32 Context Menu | M3 | T1.15 Context menu command IDs | T2.15 Menu dismiss without click | T3.15 Hook arrives with menu open | T4.15 User configuration flow |
| 16 | Corner Presets & Placement | M3 | T1.16 TL, TR, BL, BR placement | T2.16 Taskbar offset calculation | T3.16 Corner preset during wander | T4.16 Corner repositioning |
| 17 | Autostart Integration | M3 | T1.17 Registry HKCU Run set/del | T2.17 Registry key read-only/missing | T3.17 Autostart toggle cycles | T4.17 Startup launch verification |
| 18 | Standalone Hook CLI | M4 | T1.18 sidecrab-hook executable | T2.18 Invalid CLI arguments | T3.18 Hook concurrency test | T4.18 Fast CLI invocation (<10ms)|
| 19 | Hook Event Ingestion | M4 | T1.19 All 8 hook events | T2.19 Empty/malformed stdin | T3.19 Out-of-order hook events | T4.19 Claude Code lifecycle |
| 20 | State Persistence & IPC | M4 | T1.20 state.json atomic write | T2.20 Concurrent read/write race | T3.20 Missing .sidecrab directory | T4.20 Real-time IPC notification |
| 21 | Host Process Detection | M4 | T1.21 Parent PID/HWND resolution | T2.21 Headless/detached session | T3.21 Nested shell process walk | T4.21 Focus active terminal |
| 22 | Idempotent Hook Installer | M4 | T1.22 Install into settings.json | T2.22 0-byte settings.json | T3.22 Install with existing hooks | T4.22 Reinstall idempotency |
| 23 | Surgical Hook Uninstaller | M4 | T1.23 Remove sidecrab hooks | T2.23 Uninstall when no hooks exist | T3.23 Preserve third-party tools | T4.23 Clean removal check |
| 24 | E2E Opaque-Box Test Suite | E2E | T1.24 Test harness self-check | T2.24 Harness timeout handling | T3.24 Multi-tier batch run | T4.24 Full master suite run |
| 25 | Automated RAM Benchmark | M5 | T1.25 Single-sample RAM query | T2.25 Spurious peak detection | T3.25 RAM under 50 rapid events | T4.25 30s continuous benchmark |
| 26 | CPU Idle Benchmark | M5 | T1.26 CPU counter acquisition | T2.26 High-frequency timer jitter | T3.26 CPU during micro-idles | T4.26 Sustained idle CPU <=0.2% |
| 27 | Hook Simulation Test Suite | M5 | T1.27 Hook simulation runner | T2.27 Mock stdin EOF/EPIPE | T3.27 Multi-session contention | T4.27 Simulated Claude turn |
| 28 | Adversarial Hardening | M5 | T1.28 Strict schema enforcement | T2.28 Giant 100KB payload | T3.28 Rapid kill and restart | T4.28 Robustness under noise |
| 29 | Self-Contained Release Build | M5 | T1.29 Zero-dependency check | T2.29 Path with spaces/unicode | T3.29 Portable execution | T4.29 Clean production build |

---

## 4. Test Runner Commands & Execution Matrix

### 4.1 Master Test Runner
The master runner executes all suites and outputs an aggregated pass/fail summary:

```powershell
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\run_all_tests.ps1"
```

Options:
- `-IncludeBenchmark`: Runs the 30-second RAM benchmark and CPU benchmark in addition to Tiers 1-4.
- `-Tier <1|2|3|4>`: Runs only the specified tier.
- `-Verbose`: Displays detailed trace logs for every test assertion.
- `-ArtifactDir <path>`: Specifies custom build output directory.

### 4.2 Individual Test Tier Execution
Each tier can be executed independently for focused debugging:

```powershell
# Tier 1: Feature Coverage (>=5 test cases per feature)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier1_feature_coverage.ps1"

# Tier 2: Boundary & Corner Cases
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier2_boundaries_corners.ps1"

# Tier 3: Cross-Feature Combinations
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier3_cross_combinations.ps1"

# Tier 4: Real-World Application Scenarios
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier4_real_world_scenarios.ps1"

# Claude Code Hook Simulation Suite
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\hook_sim.ps1"

# 30-Second RAM Benchmark (<10MB limit enforcement)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\ram_benchmark.ps1"

# Idle CPU Benchmark (<=0.2% limit enforcement)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\cpu_benchmark.ps1"
```

---

## 5. Pass/Fail Semantics & Exit Codes

Every test script follows strict, standardized exit codes:

| Exit Code | Semantic Meaning | Description |
|-----------|------------------|-------------|
| `0` | **PASS** | All test assertions passed without regression. |
| `1` | **FAIL: Assertion Failure** | One or more test assertions failed (e.g. incorrect state, invalid JSON). |
| `2` | **FAIL: Memory Budget Exceeded** | Working set exceeded 10MB during execution. |
| `3` | **FAIL: CPU Budget Exceeded** | Idle CPU exceeded 0.2% baseline. |
| `4` | **FAIL: Engine Violation** | Forbidden browser engine process detected (WebView2, Electron, Node). |
| `5` | **FAIL: Build / Binary Missing** | Required binaries (`sidecrab-pet.exe`, `sidecrab-hook.exe`) not found. |
| `10` | **SKIP: Preconditions Unmet** | Environment lacks necessary hardware or display context. |

---

## 6. Test Isolation & Sandbox Safety

To guarantee that running automated tests never pollutes or corrupts the developer's live environment:
1. **Isolated Home Directory (`SIDECRAB_HOME`)**:
   Tests run inside an isolated temporary sandbox (`$env:TEMP/sidecrab_sandbox_<run_id>`), setting `SIDECRAB_HOME` and mocking `%USERPROFILE%/.sidecrab`.
2. **Settings.json Guard**:
   When testing settings integration, tests construct an isolated Claude settings sandbox. Any test touching the real `%USERPROFILE%/.claude/settings.json` is wrapped in guaranteed `try { ... } finally { ... }` blocks that immediately restore the pre-test state.
3. **Process Cleanup**:
   Every test harness run tracks spawned process IDs and guarantees termination (`Stop-Process -Force`) upon test completion, preventing zombie mascot instances.
4. **Registry Sandbox**:
   Autostart tests clean up any test entries created under `HKCU:\Software\Microsoft\Windows\CurrentVersion\Run`.
