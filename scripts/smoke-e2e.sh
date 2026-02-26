#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Alan E2E Smoke Test
# Needs a valid LLM API key in ~/.alan/.env or environment
# ============================================================

ALAN_BIN="./target/debug/alan"
PASS=0
FAIL=0

log_pass() { echo "✅ PASS: $1"; ((PASS++)); }
log_fail() { echo "❌ FAIL: $1"; echo "   Detail: $2"; ((FAIL++)); }
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
if [ ! -f "$HOME/.alan/.env" ] && [ -z "${ANTHROPIC_API_KEY:-}" ] && [ -z "${OPENAI_API_KEY:-}" ] && [ -z "${GEMINI_API_KEY:-}" ]; then
    log_skip "No LLM config found (~/.alan/.env or API key env vars)"
    log_info "Results: $PASS passed, $FAIL failed"
    exit 0
fi

# --- Ensure daemon is running ---
log_info "Ensuring daemon is running..."
$ALAN_BIN daemon start 2>&1 || true
sleep 2

# --- Test 1: Simple question ---
log_info "Test: simple question (alan ask)"
RESPONSE=$(timeout 30 $ALAN_BIN ask "Reply with exactly: SMOKE_OK" 2>&1) || true

if echo "$RESPONSE" | grep -q "SMOKE_OK"; then
    log_pass "alan ask: got expected response"
else
    log_fail "alan ask: unexpected response" "$RESPONSE"
fi

# --- Test 2: Math question ---
log_info "Test: non-empty response"
RESPONSE2=$(timeout 30 $ALAN_BIN ask "What is 2+2? Reply with just the number." 2>&1) || true

if [ -n "$RESPONSE2" ] && echo "$RESPONSE2" | grep -q "4"; then
    log_pass "alan ask: math question answered correctly"
else
    log_fail "alan ask: math question" "$RESPONSE2"
fi

# --- Summary ---
echo ""
echo "========================================="
echo "Results: $PASS passed, $FAIL failed"
echo "========================================="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
