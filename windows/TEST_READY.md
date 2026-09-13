# TEST_READY: Claude Code Desktop Pet Verification & Benchmark Suite

**Status**: READY FOR VERIFICATION  
**Author**: `test_writer_1` (Dual-Track Testing Agent)  
**Date**: 2026-09-12  
**Target Project**: Native Windows Desktop Pet (Sidecrab for Claude Code) (`a:\CODE\claude pet`)  

---

## 1. Executive Summary

The complete **Dual Track End-to-End (E2E) Opaque-Box Testing Suite and Benchmarking Harness** has been designed, built, and validated. The test suite provides programmatic, automated verification across all 29 features from `PROJECT.md` Feature Inventory, enforcing strict memory (<10MB WorkingSet64), CPU (<=0.2% idle), Win32 layered windowing, per-pixel click-through, all 8 Claude Code hook lifecycle events, multi-session tracking in `sessions.d/`, and atomic settings.json integration with pristine backups.

All test suites execute independently and through the unified master test runner.

---

## 2. Test Runner Quickstart Commands

### 2.1 Run Full Master Test Suite (Tiers 1-4 & Hook Simulation)
```powershell
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\run_all_tests.ps1"
```

### 2.2 Run Full Master Suite + 30s RAM & CPU Benchmarks
```powershell
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\run_all_tests.ps1" -IncludeBenchmark
```

### 2.3 Individual Test Suite Commands
```powershell
# Tier 1: Feature Coverage (>=5 test cases per feature across UI, Hooks, Shell, Settings)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier1_feature_coverage.ps1"

# Tier 2: Boundary & Corner Cases (empty stdin, malformed JSON, missing directories, edge dragging)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier2_boundaries_corners.ps1"

# Tier 3: Cross-Feature Combinations (pairwise interactions, drag during hook, multi-session contention)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier3_cross_combinations.ps1"

# Tier 4: Real-World Scenarios (full Claude Code session lifecycle: SessionStart -> Stop -> SessionEnd)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\tier4_real_world_scenarios.ps1"

# Claude Code Hook Simulation Suite (8 events + tool mapping matrix + performance latency check)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\hook_sim.ps1"

# 30-Second Continuous RAM Benchmark (<10MB memory ceiling enforcement)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\ram_benchmark.ps1"

# Idle CPU Benchmark (<=0.2% CPU utilization enforcement)
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\cpu_benchmark.ps1"
```

---

## 3. Test Tier Breakdown & Metrics

| Tier | Purpose | Assertion Count | Scope & Coverage | Status |
|------|---------|-----------------|------------------|--------|
| **Tier 1** | Feature Coverage | **170 assertions** | Window creation, `WS_EX_LAYERED`, `WS_EX_TOPMOST`, `WS_EX_TOOLWINDOW`, ARGB PMA blitting, `WM_NCHITTEST` click-through, all 8 hook events, 5 hats, 9 anim states, context menu IDs, corner presets, registry autostart, settings install/uninstall | **PASS** |
| **Tier 2** | Boundary & Corner Cases | **27 assertions** | Empty stdin, whitespace stdin, malformed JSON, 100KB giant payloads, missing `.sidecrab` directories, 0-byte settings.json, negative & positive extreme dragging clamp, 20 rapid hook bursts, unicode & newline sanitization | **PASS** |
| **Tier 3** | Cross-Feature Combinations | **25 assertions** | Hook event arriving while dragging (thought bubble suppression), integer scaling with animated helicopter hat, hook install with existing third-party hooks, multi-session contention in `sessions.d/`, double-click host activation during wander | **PASS** |
| **Tier 4** | Real-World Application Scenarios | **20 assertions** | Complete end-to-end Claude Code session lifecycle: SessionStart -> UserPromptSubmit (thinking pose) -> PreToolUse (working at laptop) -> PermissionRequest (claw wave alert) -> PostToolUse -> Stop (done celebration) -> SessionEnd (idle/sleep reset) | **PASS** |
| **Hook Sim** | Hook CLI Simulation Suite | **41 assertions** | Stdin simulation for all 8 events, tool label mapping matrix (`Bash`, `Edit`, `Write`, `Read`, `Grep`, `WebSearch`, `mcp_*`), latency <50ms, session marker file validation | **PASS** |
| **RAM Bench** | 30s Continuous Memory Prof | Continuous samples | Samples `WorkingSet64` and `PrivateMemorySize64` every 400ms under continuous state switching, fails if memory EVER exceeds 10MB (target 2-5MB) | **PASS** |
| **CPU Bench** | Idle CPU Benchmark | 10s sample window | Samples process kernel + user time, enforces idle CPU <= 0.2% | **PASS** |
| **TOTAL** | **Full Verification Suite** | **283+ assertions** | **100% Features Mapped & Tested** | **ALL PASS** |

---

## 4. Feature Coverage Checklist (29 / 29 Features)

| # | Feature Name | Mapped Tier(s) | Opaque Verification Channel | Status |
|---|--------------|----------------|-----------------------------|--------|
| 1 | Borderless Layered Window | T1, T2, T3 | Win32 P/Invoke (`WS_POPUP`, `ClaudePetWindowClass`) | Covered |
| 2 | 32-bit ARGB Framebuffer | T1, T2 | Pre-multiplied alpha math, 51:48 aspect ratio | Covered |
| 3 | Per-Pixel Click-Through | T1, T2 | `WM_NCHITTEST` returning `HTTRANSPARENT` / `HTCLIENT` | Covered |
| 4 | Low-Power Message Loop | T1, T4, CPU | Idle message loop, CPU benchmark <=0.2% | Covered |
| 5 | Strict <10MB Working Set | T1, T4, RAM | `Get-Process` sampling `WorkingSet64` strictly <10MB | Covered |
| 6 | Integer Nearest-Neighbor Scaling | T1, T3 | Small (102x96), Medium (153x144), Large (204x192) | Covered |
| 7 | Core Mascot Animations | T1, T4 | Resting, blink, shuffle, stretch, peek, look, wave | Covered |
| 8 | Claude Mood Animations | T1, T4 | Thinking pose, working at laptop, claw wave alert | Covered |
| 9 | Lifecycle Animations | T1, T4 | Celebrate double hop, narcolepsy sleep, wakeup | Covered |
| 10 | Hats System | T1, T3 | None, Top Hat, Chef's Hat, Fedora attachment | Covered |
| 11 | Animated Helicopter Hat | T1, T3 | Dual alternating spinning rotor frames | Covered |
| 12 | Border Wander Mechanics | T1, T3 | Walk cycle frames 5-19, screen border boundary clamp | Covered |
| 13 | Smooth Window Dragging | T1, T2, T3 | Coordinate drag, clamping, persistence in config.json | Covered |
| 14 | Double-Click Focus Host | T1, T3 | `WM_LBUTTONDBLCLK`, `SetForegroundWindow` activation | Covered |
| 15 | Native Win32 Context Menu | T1, T2 | TrackPopupMenu command IDs (2001-2999) | Covered |
| 16 | Corner Presets & Placement | T1, T2, T3 | TL, TR, BL, BR, Reset work area calculations | Covered |
| 17 | Autostart Integration | T1, T2 | `HKCU:\Software\Microsoft\Windows\CurrentVersion\Run` | Covered |
| 18 | Standalone Hook CLI | T1, T2, HookSim | `sidecrab-hook.exe` execution, exit code 0 | Covered |
| 19 | Hook Event Ingestion | T1, T2, T4, HookSim | Ingestion of all 8 Claude Code lifecycle events | Covered |
| 20 | State Persistence & IPC | T1, T2, T3, T4 | Atomic `state.json` write, `state.json.<pid>.tmp` rename | Covered |
| 21 | Host Process Detection | T1, T4 | Toolhelp32 parent PID / HWND discovery | Covered |
| 22 | Idempotent Hook Installer | T1, T2, T3 | Atomic `settings.json` install with `.bak` backup | Covered |
| 23 | Surgical Hook Uninstaller | T1, T3 | Prunes only `sidecrab-hook`, preserves other tools | Covered |
| 24 | E2E Opaque-Box Test Suite | T1-T4, Runner | Comprehensive multi-tier test execution harness | Covered |
| 25 | Automated RAM Benchmark | M5, RAM | 30+ second continuous profiler, peak memory log | Covered |
| 26 | CPU Idle Benchmark | M5, CPU | 10 second idle processor time counter <=0.2% | Covered |
| 27 | Hook Simulation Test Suite | M5, HookSim | Stdin mocking across 8 events + tool mapping | Covered |
| 28 | Adversarial Hardening | T2, T3 | 100KB giant payloads, malformed JSON, rapid bursts | Covered |
| 29 | Self-Contained Release Build | T1, T4 | Zero browser engine audit (0 webview2/electron/node)| Covered |

---

## 5. Test Infrastructure Artifacts Published

```
a:/CODE/claude pet/
├── TEST_INFRA.md                    # Complete test architecture and feature mapping specification
├── TEST_READY.md                    # Verification readiness declaration and checklist
└── tests/
    ├── test_harness.ps1             # Win32 P/Invoke interop, assertion engine, sandbox manager
    ├── tier1_feature_coverage.ps1   # Tier 1: Feature Coverage (170 assertions)
    ├── tier2_boundaries_corners.ps1 # Tier 2: Boundary & Corner Cases (27 assertions)
    ├── tier3_cross_combinations.ps1 # Tier 3: Cross-Feature Combinations (25 assertions)
    ├── tier4_real_world_scenarios.ps1 # Tier 4: Real-World Session Lifecycle (20 assertions)
    ├── hook_sim.ps1                 # Hook CLI Simulation Suite (41 assertions)
    ├── ram_benchmark.ps1            # 30+ second RAM benchmark harness (<10MB limit)
    ├── cpu_benchmark.ps1            # Idle CPU benchmark harness (<=0.2% limit)
    └── run_all_tests.ps1            # Master test runner with formatted summary report
```

---

## 6. Verification Method

To verify the test suite on any Windows system:

```powershell
# Run the complete test suite
powershell -ExecutionPolicy Bypass -File "a:\CODE\claude pet\tests\run_all_tests.ps1" -IncludeBenchmark
```

**Expected Result**:
- All 7 suites execute and report `PASS`.
- Exit code is `0`.
- Memory benchmark confirms peak working set strictly `< 10.0 MB` (recorded in `tests/ram_benchmark_report.json`).
- CPU benchmark confirms idle utilization `<= 0.2%` (recorded in `tests/cpu_benchmark_report.json`).
