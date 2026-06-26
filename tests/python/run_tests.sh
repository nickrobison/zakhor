#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Zakhor Python Integration Test Runner
#
# Runs the Python integration test suite against a running Zakhor MCP server.
# Tests use pytest-asyncio and communicate with the server over HTTP/SSE.
#
# Prerequisites:
#   - Rust debug build of zakhor (cargo build) available at target/debug/zakhor
#   - Python 3.12+ with uv installed
#   - GNOME Tracker 3 libraries (for tracker-rs FFI)
#   - A running Tracker SPARQL endpoint (tracker3 endpoint)
#
# Usage:
#   ./tests/python/run_tests.sh              # full suite
#   ./tests/python/run_tests.sh -k "traverse" # filter by keyword
#   ./tests/python/run_tests.sh -- -x        # pass extra args to pytest
# ---------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INTEGRATION_DIR="$SCRIPT_DIR/tests/integration"

# ---------------------------------------------------------------------------
# Colors for output
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info()  { echo -e "${CYAN}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------
FAIL=0

# Check zakhor binary exists (debug build)
ZAKHOR_BIN="$PROJECT_ROOT/target/debug/zakhor"
if [[ ! -x "$ZAKHOR_BIN" ]]; then
    log_error "zakhor binary not found at $ZAKHOR_BIN"
    log_info "Run 'cargo build' first to compile the debug binary."
    FAIL=1
fi

# Check Python 3.12+
if command -v python3 &>/dev/null; then
    PYTHON="python3"
elif command -v python &>/dev/null; then
    PYTHON="python"
else
    log_error "Python not found"
    FAIL=1
fi

if [[ $FAIL -eq 0 ]]; then
    pyver=$("$PYTHON" --version 2>&1 | grep -oP '\d+\.\d+')
    major="${pyver%.*}"
    minor="${pyver#*.}"
    if [[ "$major" -lt 3 ]] || { [[ "$major" -eq 3 ]] && [[ "$minor" -lt 12 ]]; }; then
        log_error "Python 3.12+ required, found $pyver"
        FAIL=1
    fi
fi

# Check uv
if ! command -v uv &>/dev/null; then
    log_error "uv not found — install it: https://docs.astral.sh/uv/#installation"
    FAIL=1
fi

# Check integration test directory
if [[ ! -d "$INTEGRATION_DIR" ]]; then
    log_error "Integration test directory not found: $INTEGRATION_DIR"
    FAIL=1
fi

if [[ $FAIL -ne 0 ]]; then
    echo ""
    log_error "Pre-flight checks failed — see above."
    exit 1
fi

log_ok "All prerequisites satisfied"

# ---------------------------------------------------------------------------
# Install dependencies
# ---------------------------------------------------------------------------
log_info "Installing Python dependencies..."
cd "$SCRIPT_DIR"
uv pip install -e . > /dev/null 2>&1 || uv pip install --system -e . > /dev/null 2>&1
log_ok "Dependencies installed"

# ---------------------------------------------------------------------------
# Parse extra pytest args
# ---------------------------------------------------------------------------
PYTEST_ARGS=()
if [[ $# -gt 0 ]]; then
    # If the first argument starts with `--`, pass everything to pytest
    if [[ "$1" == "--" ]]; then
        shift
        PYTEST_ARGS+=("$@")
    else
        PYTEST_ARGS+=("-k" "$*")
    fi
fi

# ---------------------------------------------------------------------------
# Run tests
# ---------------------------------------------------------------------------
log_info "Running integration tests..."
echo ""

set +e
uv run pytest "$INTEGRATION_DIR" -v "${PYTEST_ARGS[@]}"
EXIT_CODE=$?
set -e

echo ""
if [[ $EXIT_CODE -eq 0 ]]; then
    log_ok "All integration tests passed"
else
    log_error "Some integration tests failed (exit code: $EXIT_CODE)"
fi

# ---------------------------------------------------------------------------
# Clean up ephemeral databases left by tests
# ---------------------------------------------------------------------------
log_info "Cleaning up ephemeral databases..."
rm -rf /tmp/pytest-zakhor-* /tmp/zakhor-ephemeral-* 2>/dev/null || true
log_ok "Cleanup complete"

exit $EXIT_CODE
