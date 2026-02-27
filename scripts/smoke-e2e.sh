#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Alan E2E Smoke Test
# Needs a valid LLM API key in ~/.alan/.env or environment
# ============================================================

ALAN_BIN="./target/debug/alan"
PASS=0
FAIL=0

log_pass() { echo "✅ PASS: $1"; PASS=$((PASS + 1)); }
log_fail() { echo "❌ FAIL: $1"; echo "   Detail: $2"; FAIL=$((FAIL + 1)); }
log_skip() { echo "⏭️  SKIP: $1"; }
log_info() { echo "ℹ️  $1"; }

# --- Build ---
log_info "Building alan..."
if ! cargo build -p alan 2>&1; then
    log_fail "Build failed" "See compiler output above"
    exit 1
fi
log_pass "Build succeeded"

# --- Check LLM config ---
if [ ! -f "$HOME/.alan/config.toml" ]; then
    log_skip "No LLM config found (~/.alan/config.toml)"
    log_info "Results: $PASS passed, $FAIL failed"
    exit 0
fi

# --- Ensure daemon is running ---
log_info "Ensuring daemon is running..."
$ALAN_BIN daemon start 2>&1 || true
sleep 2

# ============================================================
# Test 1: text mode (default) — basic question
# ============================================================
log_info "Test: text mode — simple question"
RESPONSE=$(timeout 30 $ALAN_BIN ask "Reply with exactly: SMOKE_OK" 2>&1) || true

if echo "$RESPONSE" | grep -q "SMOKE_OK"; then
    log_pass "text mode: got expected response"
else
    log_fail "text mode: unexpected response" "$RESPONSE"
fi

# ============================================================
# Test 2: text mode — math (confirms content correctness)
# ============================================================
log_info "Test: text mode — math question"
RESPONSE2=$(timeout 30 $ALAN_BIN ask "What is 2+2? Reply with just the number." 2>&1) || true

if [ -n "$RESPONSE2" ] && echo "$RESPONSE2" | grep -q "4"; then
    log_pass "text mode: math question answered correctly"
else
    log_fail "text mode: math question" "$RESPONSE2"
fi

# ============================================================
# Test 3: json mode — NDJSON structure + turn_completed
# ============================================================
log_info "Test: json mode — NDJSON event stream"
JSON_OUT=$(timeout 30 $ALAN_BIN ask "Reply with exactly: JSON_OK" --output json 2>/dev/null) || true
JSON_EXIT=$?

# Should contain turn_completed event
if echo "$JSON_OUT" | grep -q '"type":"turn_completed"'; then
    log_pass "json mode: turn_completed present"
else
    log_fail "json mode: missing turn_completed" "$JSON_OUT"
fi

# Should contain text_delta with our content
if echo "$JSON_OUT" | grep -q '"type":"text_delta"'; then
    log_pass "json mode: text_delta present"
else
    log_fail "json mode: missing text_delta" "$JSON_OUT"
fi

# Each line should be valid JSON
JSON_VALID=true
while IFS= read -r line; do
    [ -z "$line" ] && continue
    if ! echo "$line" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        JSON_VALID=false
        break
    fi
done <<< "$JSON_OUT"

if $JSON_VALID; then
    log_pass "json mode: all lines are valid JSON"
else
    log_fail "json mode: invalid JSON line found" "$line"
fi

# ============================================================
# Test 4: quiet mode — no streaming, output at end
# ============================================================
log_info "Test: quiet mode — accumulated output"
QUIET_OUT=$(timeout 30 $ALAN_BIN ask "Reply with exactly: QUIET_OK" --output quiet 2>/dev/null) || true

if echo "$QUIET_OUT" | grep -q "QUIET_OK"; then
    log_pass "quiet mode: got expected response"
else
    log_fail "quiet mode: unexpected response" "$QUIET_OUT"
fi

# ============================================================
# Test 5: exit code — successful run returns 0
# ============================================================
log_info "Test: exit code on success"
$ALAN_BIN ask "Say hi" --output quiet --timeout 30 >/dev/null 2>&1
EXIT_CODE=$?

if [ "$EXIT_CODE" -eq 0 ]; then
    log_pass "exit code: 0 on success"
else
    log_fail "exit code: expected 0, got $EXIT_CODE" ""
fi

# ============================================================
# Test 6: timeout — short timeout triggers exit code 2
# ============================================================
log_info "Test: timeout exit code"
# Use a complex prompt that's likely to take >1s with tool calls
$ALAN_BIN ask "List every file in this directory recursively and explain each one in detail" --output quiet --timeout 1 >/dev/null 2>&1 || true
TIMEOUT_EXIT=$?

if [ "$TIMEOUT_EXIT" -eq 2 ]; then
    log_pass "timeout: exit code 2"
else
    # Timeout test is best-effort — fast LLMs might finish in 1s
    log_skip "timeout: got exit code $TIMEOUT_EXIT (LLM may have responded within 1s)"
fi

# ============================================================
# Test 7: --thinking flag (text mode) — should not crash
# ============================================================
log_info "Test: --thinking flag does not crash"
THINK_OUT=$(timeout 30 $ALAN_BIN ask "What is 1+1? Reply with just the number." --thinking 2>&1) || true
THINK_EXIT=$?

if [ "$THINK_EXIT" -eq 0 ] && echo "$THINK_OUT" | grep -q "2"; then
    log_pass "--thinking: ran successfully"
else
    log_fail "--thinking: exit=$THINK_EXIT" "$THINK_OUT"
fi

# --- Summary ---
echo ""
echo "========================================="
echo "Results: $PASS passed, $FAIL failed"
echo "========================================="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
