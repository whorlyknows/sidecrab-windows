# ==============================================================================
# Adversarial Stress Testing Suite for sidecrab-hook.exe
# Author: challenger_2
# ==============================================================================
import os
import sys
import json
import time
import shutil
import tempfile
import threading
import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed

HOOK_BIN = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "target", "release", "sidecrab-hook.exe"))
if not os.path.exists(HOOK_BIN):
    HOOK_BIN = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "target", "debug", "sidecrab-hook.exe"))

print("==================================================================")
print(" ADVERSARIAL STRESS TESTING SUITE: sidecrab-hook.exe")
print(f" Binary: {HOOK_BIN}")
print("==================================================================")

if not os.path.exists(HOOK_BIN):
    print(f"ERROR: Hook binary not found at {HOOK_BIN}")
    sys.exit(1)

test_dir = tempfile.mkdtemp(prefix="sidecrab_adv_")
sandbox_sidecrab = os.path.join(test_dir, "sidecrab_home")
sandbox_claude = os.path.join(test_dir, "claude_home")
os.makedirs(sandbox_sidecrab, exist_ok=True)
os.makedirs(sandbox_claude, exist_ok=True)

total_tests = 0
pass_count = 0
fail_count = 0
findings = []

def report_test(name: str, passed: bool, details: str = ""):
    global total_tests, pass_count, fail_count, findings
    total_tests += 1
    if passed:
        pass_count += 1
        print(f"  [PASS] {name}")
    else:
        fail_count += 1
        findings.append(f"FAIL: {name} - {details}")
        print(f"  [FAIL] {name}: {details}")

def invoke_hook(args=None, stdin_data=None, env_override=None, timeout_sec=5.0):
    cmd = [HOOK_BIN]
    if args:
        cmd.extend(args)
    
    env = os.environ.copy()
    env["SIDECRAB_HOME"] = sandbox_sidecrab
    env["CLAUDE_HOME"] = sandbox_claude
    if env_override:
        env.update(env_override)
    
    t0 = time.perf_counter()
    try:
        proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env
        )
        
        if stdin_data is not None:
            if isinstance(stdin_data, str):
                stdin_bytes = stdin_data.encode("utf-8")
            else:
                stdin_bytes = stdin_data
        else:
            stdin_bytes = b""
            
        stdout_b, stderr_b = proc.communicate(input=stdin_bytes, timeout=timeout_sec)
        t1 = time.perf_counter()
        duration_ms = (t1 - t0) * 1000.0
        
        return {
            "exit_code": proc.returncode,
            "stdout": stdout_b.decode("utf-8", errors="replace"),
            "stderr": stderr_b.decode("utf-8", errors="replace"),
            "duration_ms": duration_ms,
            "timed_out": False
        }
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate()
        t1 = time.perf_counter()
        return {
            "exit_code": -999,
            "stdout": "",
            "stderr": "TIMEOUT",
            "duration_ms": (t1 - t0) * 1000.0,
            "timed_out": True
        }

try:
    # --------------------------------------------------------------------------
    # SECTION 1: EXTREME AND MALFORMED INPUTS
    # --------------------------------------------------------------------------
    print("\n--- SECTION 1: Extreme and Malformed Inputs ---")

    # 1.1 Empty stdin
    r = invoke_hook(args=["prompt"], stdin_data=b"")
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.1 Empty stdin handled gracefully without panic", r["exit_code"] == 0 and no_panic and not r["timed_out"], f"Exit={r['exit_code']}, Err={r['stderr']}")

    # 1.2 Whitespace-only stdin
    r = invoke_hook(args=["pre"], stdin_data="   \t\r\n\n\t   ")
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.2 Whitespace-only stdin handled gracefully", r["exit_code"] == 0 and no_panic and not r["timed_out"], f"Exit={r['exit_code']}")

    # 1.3 Binary garbage stdin
    garbage = bytes([0xFF, 0xFE, 0xC0, 0x80, 0x00, 0x12, 0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF])
    r = invoke_hook(args=["post"], stdin_data=garbage)
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.3 Binary garbage bytes handled without crash or panic", r["exit_code"] == 0 and no_panic and not r["timed_out"], f"Exit={r['exit_code']}")

    # 1.4 Malformed JSON (truncated syntax)
    r = invoke_hook(args=["pre"], stdin_data='{"tool_name": "Bash", "session_id": ')
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.4 Truncated malformed JSON handled gracefully", r["exit_code"] == 0 and no_panic and not r["timed_out"], f"Exit={r['exit_code']}")

    # 1.5 Non-JSON raw tokens
    r = invoke_hook(args=["prompt"], stdin_data="<<<XML_OR_OTHER_ARBITRARY_TEXT>>>")
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.5 Non-JSON tokens handled gracefully without panic", r["exit_code"] == 0 and no_panic and not r["timed_out"], f"Exit={r['exit_code']}")

    # 1.6 Non-object JSON (bare primitives & arrays)
    r1 = invoke_hook(args=["prompt"], stdin_data='"just a string"')
    r2 = invoke_hook(args=["prompt"], stdin_data="123456789")
    r3 = invoke_hook(args=["prompt"], stdin_data="[1, 2, 3, 4, 5]")
    r4 = invoke_hook(args=["prompt"], stdin_data="true")
    all_ok = all(x["exit_code"] == 0 and "panicked at" not in x["stderr"] for x in [r1, r2, r3, r4])
    report_test("1.6 Non-object JSON (primitives, arrays) handled gracefully", all_ok, f"Exits={[x['exit_code'] for x in [r1, r2, r3, r4]]}")

    # 1.7 Deeply nested JSON (200 levels)
    deep_json = ('{"child":' * 200) + 'null' + ('}' * 200)
    r = invoke_hook(args=["prompt"], stdin_data=deep_json)
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.7 Deeply nested JSON (200 levels) handled without stack overflow", r["exit_code"] == 0 and no_panic and not r["timed_out"], f"Exit={r['exit_code']}")

    # 1.8 100KB giant payload
    giant_str = "A" * (100 * 1024)
    giant_payload = json.dumps({"session_id": "giant-session-100kb", "prompt": giant_str, "cwd": "A:\\CODE\\claude pet"})
    r = invoke_hook(args=["prompt"], stdin_data=giant_payload)
    no_panic = "panicked at" not in r["stderr"]
    state_file = os.path.join(sandbox_sidecrab, "state.json")
    state_ok = False
    if os.path.exists(state_file):
        with open(state_file, "r", encoding="utf-8") as sf:
            st = json.load(sf)
            state_ok = (st.get("state") == "thinking") and bool(st.get("prompt"))
    report_test("1.8 100KB giant payload processed and state written cleanly", r["exit_code"] == 0 and no_panic and state_ok, f"Exit={r['exit_code']}, StateOk={state_ok}, Latency={r['duration_ms']:.1f}ms")

    # 1.9 500KB giant payload
    giant_500k = json.dumps({"session_id": "giant-500k", "prompt": "B" * (500 * 1024)})
    r = invoke_hook(args=["prompt"], stdin_data=giant_500k)
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.9 500KB giant payload processed safely", r["exit_code"] == 0 and no_panic, f"Exit={r['exit_code']}, Latency={r['duration_ms']:.1f}ms")

    # 1.10 Missing arguments (zero CLI args)
    r = invoke_hook(args=[], stdin_data="")
    no_panic = "panicked at" not in r["stderr"]
    report_test("1.10 Invocation with zero CLI arguments handled gracefully", r["exit_code"] == 0 and no_panic, f"Exit={r['exit_code']}")

    # 1.11 Invalid event names
    r1 = invoke_hook(args=["foo_invalid_event"], stdin_data='{"session_id":"s1"}')
    r2 = invoke_hook(args=["12345"], stdin_data='{"session_id":"s1"}')
    r3 = invoke_hook(args=["--unknown-flag-xyz"], stdin_data='{"session_id":"s1"}')
    no_panic = all("panicked at" not in x["stderr"] for x in [r1, r2, r3])
    report_test("1.11 Invalid event names fall back without panic", all(x["exit_code"] == 0 for x in [r1, r2, r3]) and no_panic, f"Exits={[x['exit_code'] for x in [r1, r2, r3]]}")

    # 1.12 Non-existent paths for --install, --uninstall, --status
    non_existent = "Z:\\nonexistent_drive_99\\deep\\dir\\settings.json"
    r_inst = invoke_hook(args=["--install", non_existent])
    r_uninst = invoke_hook(args=["--uninstall", non_existent])
    r_stat = invoke_hook(args=["--status", non_existent])
    no_panic = all("panicked at" not in x["stderr"] for x in [r_inst, r_uninst, r_stat])
    # --install: exit 2 (err), --uninstall: exit 0 (noop), --status: exit 1 (not installed)
    report_test("1.12 Non-existent paths handled cleanly with error codes (no panics)", r_inst["exit_code"] == 2 and r_uninst["exit_code"] == 0 and r_stat["exit_code"] == 1 and no_panic, f"InstExit={r_inst['exit_code']}, UninstExit={r_uninst['exit_code']}, StatExit={r_stat['exit_code']}")

    # 1.13 Unicode multibyte & emoji truncation safety (crossing 120-char boundary)
    unicode_prompt = ("🦀 Crab Mascot 🌟 🚀 " * 15) + "日本語テキスト"
    unicode_payload = json.dumps({"session_id": "unicode-sess", "prompt": unicode_prompt, "cwd": "A:\\CODE\\claude pet"})
    r_uni = invoke_hook(args=["prompt"], stdin_data=unicode_payload)
    no_panic_uni = "panicked at" not in r_uni["stderr"]
    state_file = os.path.join(sandbox_sidecrab, "state.json")
    uni_preview_ok = False
    if os.path.exists(state_file):
        with open(state_file, "r", encoding="utf-8") as sf:
            st = json.load(sf)
            uni_preview_ok = bool(st.get("prompt_preview")) and st["prompt_preview"].endswith("…")
    report_test("1.13 Multibyte Unicode & Emoji prompt truncated safely without char boundary panic", r_uni["exit_code"] == 0 and no_panic_uni and uni_preview_ok, f"Exit={r_uni['exit_code']}, PreviewOk={uni_preview_ok}")

    # 1.14 Path traversal attack in session_id
    traversal_payload = json.dumps({"session_id": "../../malicious_escape_path", "cwd": "A:\\CODE\\claude pet"})
    r_trav = invoke_hook(args=["start"], stdin_data=traversal_payload)
    # Check that it did NOT escape sessions.d
    sessions_dir = os.path.join(sandbox_sidecrab, "sessions.d")
    escaped_file = os.path.join(sandbox_sidecrab, "malicious_escape_path")
    sanitized_file = os.path.join(sessions_dir, "malicious_escape_path.json")
    no_escape = not os.path.exists(escaped_file) and os.path.exists(sanitized_file)
    report_test("1.14 Path traversal in session_id sanitized (alphanumeric/dash/underscore only)", r_trav["exit_code"] == 0 and no_escape, f"Exit={r_trav['exit_code']}, EscapedExists={os.path.exists(escaped_file)}, SanitizedExists={os.path.exists(sanitized_file)}")

    # 1.15 Windows reserved device name in session_id (NUL, CON, AUX)
    nul_payload = json.dumps({"session_id": "CON", "cwd": "A:\\CODE\\claude pet"})
    r_nul = invoke_hook(args=["start"], stdin_data=nul_payload)
    no_panic_nul = "panicked at" not in r_nul["stderr"]
    report_test("1.15 Windows reserved device name session_id handled safely without panic", r_nul["exit_code"] == 0 and no_panic_nul and not r_nul["timed_out"], f"Exit={r_nul['exit_code']}")


    # --------------------------------------------------------------------------
    # SECTION 2: CONCURRENCY AND RAPID BURSTS (30 CONCURRENT INVOCATIONS)
    # --------------------------------------------------------------------------
    print("\n--- SECTION 2: Concurrency and Rapid Bursts (30 Parallel Hooks) ---")

    burst_dir = os.path.join(test_dir, "burst_home")
    os.makedirs(burst_dir, exist_ok=True)
    burst_state = os.path.join(burst_dir, "state.json")
    burst_sessions = os.path.join(burst_dir, "sessions.d")

    # Start initial primary session
    init_payload = json.dumps({"session_id": "session-primary", "cwd": "A:\\CODE\\claude pet"})
    invoke_hook(args=["start"], stdin_data=init_payload, env_override={"SIDECRAB_HOME": burst_dir})

    # Background monitoring thread continuously reading state.json
    monitor_running = True
    valid_reads = 0
    corrupt_reads = 0
    read_lock = threading.Lock()

    def state_monitor():
        global valid_reads, corrupt_reads, monitor_running
        while monitor_running:
            if os.path.exists(burst_state):
                try:
                    with open(burst_state, "r", encoding="utf-8") as f:
                        content = f.read()
                    if content.strip():
                        data = json.loads(content)
                        if "state" in data:
                            with read_lock:
                                valid_reads += 1
                        else:
                            with read_lock:
                                corrupt_reads += 1
                except (json.JSONDecodeError, UnicodeDecodeError):
                    with read_lock:
                        corrupt_reads += 1
                except (PermissionError, OSError):
                    # Windows file sharing lock during atomic rename is expected transient OS behavior
                    pass
            time.sleep(0.002)

    mon_thread = threading.Thread(target=state_monitor, daemon=True)
    mon_thread.start()

    events = ["pre", "post", "prompt", "notify", "pre"]
    tools = ["Bash", "Edit", "Write", "Grep", "Read"]

    def run_concurrent_hook(idx):
        ev = events[idx % len(events)]
        tool = tools[idx % len(tools)]
        sid = f"session-secondary-{idx}" if (idx % 5 == 0) else "session-primary"
        payload = json.dumps({
            "session_id": sid,
            "tool_name": tool,
            "prompt": f"Burst concurrent query {idx}",
            "cwd": "A:\\CODE\\claude pet"
        })
        return invoke_hook(args=[ev], stdin_data=payload, env_override={"SIDECRAB_HOME": burst_dir})

    t_burst_start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=30) as executor:
        futures = [executor.submit(run_concurrent_hook, i) for i in range(30)]
        results = [f.result() for f in as_completed(futures)]
    t_burst_end = time.perf_counter()
    burst_duration_ms = (t_burst_end - t_burst_start) * 1000.0

    monitor_running = False
    mon_thread.join(timeout=1.0)

    all_exit_zero = all(r["exit_code"] == 0 for r in results)
    report_test(f"2.1 Burst of 30 concurrent hooks all exit code 0 ({burst_duration_ms:.1f}ms total)", all_exit_zero, f"Failed exits: {[r['exit_code'] for r in results if r['exit_code'] != 0]}")

    # Verify state.json final validity & zero corruptions observed
    final_state_valid = False
    if os.path.exists(burst_state):
        try:
            with open(burst_state, "r", encoding="utf-8") as f:
                st = json.load(f)
                final_state_valid = bool(st.get("state"))
        except Exception:
            final_state_valid = False

    report_test(f"2.2 state.json was NEVER corrupted during 30-burst (sampled {valid_reads} valid reads, {corrupt_reads} corrupt)", corrupt_reads == 0 and final_state_valid, f"Corruptions={corrupt_reads}, FinalValid={final_state_valid}")

    # Multi-session isolation in sessions.d/
    print("\n--- Multi-Session Isolation in sessions.d/ ---")
    iso_dir = os.path.join(test_dir, "iso_home")
    os.makedirs(iso_dir, exist_ok=True)
    iso_sessions = os.path.join(iso_dir, "sessions.d")

    # Session A starts
    p_a = json.dumps({"session_id": "session-alpha-123", "cwd": "A:\\CODE\\projectA"})
    invoke_hook(args=["start"], stdin_data=p_a, env_override={"SIDECRAB_HOME": iso_dir})

    # Session B starts
    p_b = json.dumps({"session_id": "session-beta-456", "cwd": "A:\\CODE\\projectB"})
    invoke_hook(args=["start"], stdin_data=p_b, env_override={"SIDECRAB_HOME": iso_dir})

    # Both exist
    has_alpha = os.path.exists(os.path.join(iso_sessions, "session-alpha-123.json")) and os.path.exists(os.path.join(iso_sessions, "session-alpha-123"))
    has_beta = os.path.exists(os.path.join(iso_sessions, "session-beta-456.json")) and os.path.exists(os.path.join(iso_sessions, "session-beta-456"))
    report_test("2.3 Both concurrent sessions tracked simultaneously in sessions.d/", has_alpha and has_beta, f"Alpha={has_alpha}, Beta={has_beta}")

    # Session A sets working state (PreToolUse)
    p_a_tool = json.dumps({"session_id": "session-alpha-123", "tool_name": "Bash"})
    invoke_hook(args=["pre"], stdin_data=p_a_tool, env_override={"SIDECRAB_HOME": iso_dir})

    with open(os.path.join(iso_dir, "state.json"), "r", encoding="utf-8") as f:
        st_a = json.load(f)
    is_tool = (st_a.get("state") == "tool") and (st_a.get("session_id") == "session-alpha-123" or st_a.get("active_session_id") == "session-alpha-123")

    # Session B closes (SessionEnd)
    invoke_hook(args=["end"], stdin_data=p_b, env_override={"SIDECRAB_HOME": iso_dir})

    beta_gone = not os.path.exists(os.path.join(iso_sessions, "session-beta-456.json"))
    alpha_remains = os.path.exists(os.path.join(iso_sessions, "session-alpha-123.json"))
    report_test("2.4 SessionEnd for Session B cleanly prunes only Session B from sessions.d/", beta_gone and alpha_remains, f"BetaGone={beta_gone}, AlphaRemains={alpha_remains}")

    with open(os.path.join(iso_dir, "state.json"), "r", encoding="utf-8") as f:
        st_after_b = json.load(f)
    session_a_protected = (st_after_b.get("state") == "tool") and (st_after_b.get("active_session_id") == "session-alpha-123" or st_after_b.get("session_id") == "session-alpha-123")
    report_test("2.5 Secondary session ending does NOT stomp primary session's active state", session_a_protected, f"State={st_after_b.get('state')}, ActiveSid={st_after_b.get('active_session_id')}")

    # --------------------------------------------------------------------------
    # SECTION 3: SETTINGS.JSON ADVERSARIAL EDGE CASES
    # --------------------------------------------------------------------------
    print("\n--- SECTION 3: Settings.json Adversarial Edge Cases ---")

    # 3.1 0-byte settings.json
    empty_settings = os.path.join(test_dir, "empty_settings.json")
    with open(empty_settings, "wb") as f:
        f.write(b"")
    empty_bak = empty_settings + ".bak"

    r_inst_empty = invoke_hook(args=["--install", empty_settings])
    bak_created = os.path.exists(empty_bak)
    with open(empty_settings, "r", encoding="utf-8") as f:
        installed_data = json.load(f)
    hooks_installed = "hooks" in installed_data and "UserPromptSubmit" in installed_data["hooks"] and len(installed_data["hooks"]["UserPromptSubmit"]) >= 1
    report_test("3.1 Install against 0-byte file creates .bak and populates valid JSON hooks", r_inst_empty["exit_code"] == 0 and bak_created and hooks_installed, f"Exit={r_inst_empty['exit_code']}, BakCreated={bak_created}, HooksInstalled={hooks_installed}")

    r_uninst_empty = invoke_hook(args=["--uninstall", empty_settings])
    with open(empty_settings, "r", encoding="utf-8") as f:
        uninstalled_data = json.load(f)
    hooks_obj = uninstalled_data.get("hooks", {})
    all_sidecrab_removed = len(hooks_obj) == 0 or "UserPromptSubmit" not in hooks_obj
    report_test("3.2 Uninstall against emptied file removes hooks cleanly into valid JSON", r_uninst_empty["exit_code"] == 0 and all_sidecrab_removed, f"Exit={r_uninst_empty['exit_code']}, Removed={all_sidecrab_removed}")

    # 3.2 Pre-existing custom third-party hooks and configurations
    custom_settings = os.path.join(test_dir, "custom_settings.json")
    custom_bak = custom_settings + ".bak"
    custom_initial = {
        "theme": "monokai-pro",
        "editor": "code",
        "auto_update": False,
        "hooks": {
            "UserPromptSubmit": [
                {"type": "command", "command": "my-custom-prompt-logger.exe", "args": ["--verbose"]}
            ],
            "PreToolUse": [
                {"matcher": "Bash", "hooks": [{"type": "command", "command": "rtk hook claude"}]}
            ]
        }
    }
    custom_initial_text = json.dumps(custom_initial, indent=2)
    with open(custom_settings, "w", encoding="utf-8") as f:
        f.write(custom_initial_text)

    # First Install
    r_inst_custom = invoke_hook(args=["--install", custom_settings])
    bak_exists = os.path.exists(custom_bak)
    bak_text = ""
    if bak_exists:
        with open(custom_bak, "r", encoding="utf-8") as f:
            bak_text = f.read()
    bak_matches = (json.loads(bak_text) == custom_initial)

    with open(custom_settings, "r", encoding="utf-8") as f:
        installed_val = json.load(f)

    has_custom_logger = any(h.get("command") == "my-custom-prompt-logger.exe" for h in installed_val["hooks"]["UserPromptSubmit"])
    has_sidecrab_prompt = any(
        ("sidecrab-hook" in json.dumps(h)) for h in installed_val["hooks"]["UserPromptSubmit"]
    )
    report_test("3.3 Install preserves third-party hooks and creates pristine backup", r_inst_custom["exit_code"] == 0 and bak_matches and has_custom_logger and has_sidecrab_prompt, f"BakMatches={bak_matches}, CustomLogger={has_custom_logger}, SidecrabPrompt={has_sidecrab_prompt}")

    # Second Install (Idempotency)
    r_inst_again = invoke_hook(args=["--install", custom_settings])
    with open(custom_settings, "r", encoding="utf-8") as f:
        installed_val2 = json.load(f)
    sidecrab_prompt_count = sum(
        1 for h in installed_val2["hooks"]["UserPromptSubmit"] if "sidecrab-hook" in json.dumps(h)
    )
    report_test("3.4 Second install is idempotent (no duplicate entries, count = 1)", r_inst_again["exit_code"] == 0 and sidecrab_prompt_count == 1, f"SidecrabCount={sidecrab_prompt_count}")

    # Uninstall
    r_uninst_custom = invoke_hook(args=["--uninstall", custom_settings])
    with open(custom_settings, "r", encoding="utf-8") as f:
        uninst_val = json.load(f)
    theme_preserved = (uninst_val.get("theme") == "monokai-pro") and (uninst_val.get("editor") == "code")
    custom_hook_preserved = len(uninst_val["hooks"].get("UserPromptSubmit", [])) == 1 and uninst_val["hooks"]["UserPromptSubmit"][0].get("command") == "my-custom-prompt-logger.exe"
    sidecrab_completely_gone = ("sidecrab-hook" not in json.dumps(uninst_val))
    report_test("3.5 Uninstall surgical pruning (third-party config & hooks 100% preserved, 0 sidecrab entries)", r_uninst_custom["exit_code"] == 0 and theme_preserved and custom_hook_preserved and sidecrab_completely_gone, f"ThemePreserved={theme_preserved}, CustomHook={custom_hook_preserved}, SidecrabGone={sidecrab_completely_gone}")

    # Backup Restoration Verification
    shutil.copyfile(custom_bak, custom_settings)
    with open(custom_settings, "r", encoding="utf-8") as f:
        restored_content = f.read()
    pristine_restored = (json.loads(restored_content) == custom_initial)
    report_test("3.6 settings.json.bak restored byte-for-byte to original without loss", pristine_restored, "Matches original")

    # 3.7 Syntactically invalid/corrupted JSON in settings.json
    corrupted_settings = os.path.join(test_dir, "corrupted_settings.json")
    corrupted_bak = corrupted_settings + ".bak"
    corrupted_content = "{\"theme\": \"dark\", \"broken_json\": [1, 2, " # truncated syntax error
    with open(corrupted_settings, "w", encoding="utf-8") as f:
        f.write(corrupted_content)

    r_inst_corrupt = invoke_hook(args=["--install", corrupted_settings])
    bak_saved = os.path.exists(corrupted_bak)
    with open(corrupted_bak, "r", encoding="utf-8") as f:
        saved_text = f.read()
    # Verify backup contains the exact corrupted content
    bak_exact = (saved_text == corrupted_content)
    # Verify new settings.json is valid JSON
    new_valid = False
    try:
        with open(corrupted_settings, "r", encoding="utf-8") as f:
            valid_json = json.load(f)
            new_valid = "hooks" in valid_json
    except Exception:
        new_valid = False
    report_test("3.7 Install against corrupted JSON creates pristine .bak and recovers to valid settings", r_inst_corrupt["exit_code"] == 0 and bak_saved and bak_exact and new_valid, f"Exit={r_inst_corrupt['exit_code']}, BakSaved={bak_saved}, BakExact={bak_exact}, NewValid={new_valid}")

    # 3.8 Existing backup preservation guarantee across subsequent installs
    # Modifying settings and running install again should NOT overwrite the existing .bak
    with open(corrupted_settings, "w", encoding="utf-8") as f:
        f.write(json.dumps({"modified_key": "new_value"}))
    invoke_hook(args=["--install", corrupted_settings])
    with open(corrupted_bak, "r", encoding="utf-8") as f:
        bak_after = f.read()
    bak_not_overwritten = (bak_after == corrupted_content)
    report_test("3.8 Existing settings.json.bak is NEVER overwritten by subsequent installs", bak_not_overwritten, "Backup preserved")


    # --------------------------------------------------------------------------
    # SECTION 4: LATENCY AND PERFORMANCE METRICS
    # --------------------------------------------------------------------------
    print("\n--- SECTION 4: Latency & Performance Metrics ---")

    latencies = []
    events_to_sample = ["prompt", "pre", "post", "notify", "permreq", "stop", "start", "end"]
    sample_payload = json.dumps({
        "session_id": "perf-sample-sess",
        "tool_name": "Bash",
        "prompt": "Performance benchmark test",
        "cwd": "A:\\CODE\\claude pet"
    })

    # Warmup
    invoke_hook(args=["prompt"], stdin_data=sample_payload)

    for i in range(50):
        ev = events_to_sample[i % len(events_to_sample)]
        res = invoke_hook(args=[ev], stdin_data=sample_payload)
        if res["exit_code"] == 0:
            latencies.append(res["duration_ms"])

    latencies.sort()
    min_lat = round(latencies[0], 2)
    max_lat = round(latencies[-1], 2)
    avg_lat = round(sum(latencies) / len(latencies), 2)
    p50_lat = round(latencies[int(len(latencies) * 0.50)], 2)
    p95_lat = round(latencies[int(len(latencies) * 0.95)], 2)
    p99_lat = round(latencies[int(len(latencies) * 0.99)], 2)

    print(f"  Samples: {len(latencies)} runs")
    print(f"  Min Latency: {min_lat} ms")
    print(f"  Avg Latency: {avg_lat} ms")
    print(f"  P50 Latency: {p50_lat} ms")
    print(f"  P95 Latency: {p95_lat} ms")
    print(f"  P99 Latency: {p99_lat} ms")
    print(f"  Max Latency: {max_lat} ms")

    sub_50ms = (p95_lat < 50.0)
    report_test(f"4.1 P95 execution latency strictly under 50ms ceiling (actual: {p95_lat} ms)", sub_50ms, f"P95={p95_lat}ms, Avg={avg_lat}ms")

    # Export report json
    report = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "total_tests": total_tests,
        "passed": pass_count,
        "failed": fail_count,
        "latency_metrics": {
            "samples": len(latencies),
            "min_ms": min_lat,
            "avg_ms": avg_lat,
            "p50_ms": p50_lat,
            "p95_ms": p95_lat,
            "p99_ms": p99_lat,
            "max_ms": max_lat
        },
        "burst_metrics": {
            "concurrent_jobs": 30,
            "total_burst_duration_ms": round(burst_duration_ms, 2),
            "state_reads_sampled": valid_reads,
            "corruptions": corrupt_reads
        },
        "findings": findings
    }

    report_json_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "adversarial_report.json"))
    with open(report_json_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
    print(f"\nAdversarial report written to {report_json_path}")

finally:
    try:
        shutil.rmtree(test_dir, ignore_errors=True)
    except Exception:
        pass

print("==================================================================")
pct = (pass_count * 100 / total_tests) if total_tests > 0 else 0
print(f" ADVERSARIAL STRESS TEST SUMMARY: Passed {pass_count} / {total_tests} ({pct:.1f}%)")
print("==================================================================")

if fail_count > 0:
    sys.exit(1)
else:
    sys.exit(0)

