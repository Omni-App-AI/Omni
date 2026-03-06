# App Interact Audit: Bugs Found & Fixed

Audit date: 2026-03-05
Spec: `immutable-waddling-fiddle.md` (App Interact System: Complete Rewrite Plan)
Workspace tests: 842 (was 765 before fixes)

---

## BUG 1: Missing `retry_count` / `retry_delay_ms` Parameters — **FIXED**
**Severity:** Medium | **Spec Step:** 16
**Files changed:** `mod.rs`, `platform.rs`, `stub.rs`, `windows/mod.rs`, `windows/interaction.rs`

The spec required optional `retry_count` and `retry_delay_ms` parameters so the LLM can control retry behavior for click/type_text/read_text. Was hardcoded to 3 retries / 300ms.

**Fix:** Added both params to the JSON schema, extracted in handlers with defaults (3/300) and max caps (10/2000), threaded through trait methods into `wait_for_ready()`.

---

## BUG 2: Misleading Docstring in `element_ref.rs` — **FIXED**
**Severity:** Low | **Spec Step:** 7
**File changed:** `windows/element_ref.rs:110-112`

Docstring said "Tier 1: RuntimeId lookup" but code uses HWND as Tier 1 (correct engineering decision since RuntimeId requires full tree walk). Comment was misleading.

**Fix:** Updated docstring to match actual implementation: Tier 1 = HWND + criteria, Tier 2 = Legacy hash, Tier 3 = Global search.

---

## BUG 3: Comment/Code Mismatch on Click Cascade — **FIXED**
**Severity:** Cosmetic | **Spec Step:** 9
**File changed:** `windows/interaction.rs:71`

Function doc said "5-step pattern cascade" but implementation has 7 steps (foreground + 5 semantic patterns + coordinate fallback).

**Fix:** Updated comment to "7-step pattern cascade".

---

## BUG 4: Zero Tests in `interaction.rs` — **FIXED**
**Severity:** High | **Spec Step:** 17
**File changed:** `windows/interaction.rs`

The most critical file (click/type/read logic, wait_for_ready retry) had zero unit tests. Spec explicitly required wait_for_ready retry logic tests.

**Fix:** Added 4 tests: default constants, invalid ref failure, zero retries fast fail, custom retry params.

---

## BUG 5: Zero Tests in `screenshot.rs` — **FIXED**
**Severity:** Medium | **Spec Step:** 17
**File changed:** `windows/screenshot.rs`

No tests for the 3-tier GDI capture cascade or PNG encoding.

**Fix:** Added 4 tests: PNG encoding with valid pixels, 1x1 pixel encoding, screenshot constants, BGRA→RGBA conversion.

---

## BUG 6: Missing RuntimeId Encoding Roundtrip Test — **FIXED**
**Severity:** Medium | **Spec Step:** 17
**File changed:** `windows/element_ref.rs`

Spec explicitly required an encode→decode→verify roundtrip test.

**Fix:** Added 4 tests: full roundtrip with runtime_id, empty runtime_id handling, backward compat legacy hash format, invalid ref handling.

---

## BUG 7: Minimal Tests for `tree.rs` — **FIXED**
**Severity:** Low | **Spec Step:** 17
**File changed:** `windows/tree.rs`

Only 1 test (depth_constants). No tests for truncation behavior.

**Fix:** Added 4 tests: element cap constant, no-truncation result, element cap truncation, depth limit truncation.

---

## BUG 8: Minimal Tests for `chromium.rs` — **FIXED**
**Severity:** Low | **Spec Step:** 17
**File changed:** `windows/chromium.rs`

Only 2 basic constant tests.

**Fix:** Added 2 tests: all 3 Chromium class variants, non-Chromium class negative cases.

---

## Spec Compliance Summary (Post-Fix)

| Step | Description | Status |
|------|-------------|--------|
| 1 | Cargo.toml uiautomation features | PASS |
| 2 | ParsedElementRef struct | PASS |
| 3 | windows/ directory split | PASS |
| 4 | helpers.rs extraction | PASS |
| 5 | focus.rs 5-strategy cascade | PASS |
| 6 | dpi.rs per-monitor awareness | PASS |
| 7 | element_ref.rs 3-tier decode | PASS (fixed docstring) |
| 8 | input.rs crate-based input | PASS |
| 9 | interaction.rs retry + cascades | PASS (added tests) |
| 10 | tree.rs walk_tree_recursive | PASS (added tests) |
| 11 | screenshot.rs 3-step cascade | PASS (added tests) |
| 12 | chromium.rs browser support | PASS (added tests) |
| 13 | windows/mod.rs assembly | PASS |
| 14 | platform.rs new trait methods | PASS |
| 15 | stub.rs new stubs | PASS |
| 16 | mod.rs retry pass-through | PASS (was FAIL — fixed) |
| 17 | Tests | PASS (added 18 new tests) |

All 17 spec steps now pass. Total app_interact tests: 90 (was 72).

---

## Runtime Bugs: Edge/Chrome Browser Interaction Failures

### BUG 9: `find_element()` fails immediately when window not ready — **FIXED**
**Severity:** CRITICAL
**File changed:** `windows/mod.rs` (find_element rewritten)

`find_element()` called `helpers::find_window()` ONCE outside the polling loop. If the browser window hadn't fully initialized yet (common with Edge/Chrome which take 5-8s to start), the entire operation failed immediately with "No window found" — no retry at all.

**Fix:** Moved `find_window()` inside the polling loop with `continue` on failure. Now retries window search every 500ms until the timeout expires.

---

### BUG 10: `find_elements()` same immediate failure — **FIXED**
**Severity:** High
**File changed:** `windows/mod.rs`

Same issue as BUG 9 but in `find_elements()`.

**Fix:** Changed to use `find_window_with_retry()`.

---

### BUG 11: `launch_app()` timeout too short for browsers — **FIXED**
**Severity:** High
**File changed:** `windows/mod.rs:59`

5-second deadline insufficient for Edge/Chrome which spawn multiple processes (main, network, GPU, renderer) and may take 5-8 seconds to create a usable UIA window.

**Fix:** Increased from 5s to 10s with explanatory comment.

---

### BUG 12: `is_usable_window()` filters out Chromium windows during startup — **FIXED**
**Severity:** High
**File changed:** `windows/helpers.rs`

Edge/Chrome create temporary zero-height frames during initialization. The bounding-rect check (`w <= 0 || h <= 0`) filtered these out, causing `find_window()` to miss newly-launched browser windows.

**Fix:** Skip bounding-rect check for Chromium window classes (`Chrome_WidgetWin_1`, `Chrome_WidgetWin_0`, `CEF_BrowserWindow`).

---

### BUG 13: `focus_window()`/`get_tree()`/`press_keys()` fail on newly launched apps — **FIXED**
**Severity:** Medium
**Files changed:** `windows/mod.rs`, `windows/helpers.rs`

These methods call `find_window()` without retry. When called shortly after `launch_app()`, the window may not be ready.

**Fix:** Added `find_window_with_retry()` helper with 5s default timeout. Applied to `focus_window`, `get_tree`, `press_keys`, `find_elements`.

---

### BUG 14: `normalize_for_comparison()` missing Unicode ranges — **FIXED**
**Severity:** Medium
**File changed:** `windows/helpers.rs`

Edge inserts bidirectional markers (U+202A-202E), non-breaking spaces (U+00A0), and interlinear annotations (U+FFF9-FFFB) that weren't stripped, causing fuzzy title matching to fail.

**Fix:** Added 3 new Unicode ranges to the filter: bidi embeddings, non-breaking space, interlinear annotations.

---

### BUG 15: Chromium per-attempt timeout too short — **FIXED**
**Severity:** Medium
**Files changed:** `windows/mod.rs`, `windows/chromium.rs`

`find_element()` gave Chromium search only 500ms per attempt (same as standard). Chromium content takes 1-2s+ to appear in UIA. `find_render_widget_host()` had a hardcoded 2s timeout too short for post-launch.

**Fix:** Chromium per-attempt timeout increased to 1500ms in `find_element()`. Render widget host timeout increased from 2000ms to 3000ms.

---

## Summary

| Category | Count | Severity |
|----------|-------|----------|
| Spec deviations (Bugs 1-3) | 3 | Medium/Low/Cosmetic |
| Missing tests (Bugs 4-8) | 5 | High/Medium/Low |
| Runtime failures (Bugs 9-15) | 7 | Critical/High/Medium |
| **Total** | **15** | |

All 15 bugs fixed. 842 workspace tests pass (0 failures).
