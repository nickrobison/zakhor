#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Zakhor Python Integration Test Runner
#
# Runs the Python test suite against a running Zakhor MCP server.
#
# Collects the whole tests/python tree, not just tests/integration: passing a
# directory to pytest overrides `testpaths` in pyproject.toml, which silently
# excluded every top-level test_*.py while the suite still reported success
# (issue #84).
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

# Check zakhor binary exists (debug build). CARGO_TARGET_DIR is honoured so a
# customised target directory does not look like a missing build.
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
ZAKHOR_BIN="$CARGO_TARGET_DIR/debug/zakhor"
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

  # Check there is at least something to collect, and that everything present is
  # actually collected. pytest is invoked from SCRIPT_DIR with no path argument
  # so `testpaths` in pyproject.toml governs collection; passing a directory
  # here previously overrode it and silently excluded every top-level test_*.py
  # (issue #84).
  mapfile -t TEST_FILES < <(find "$SCRIPT_DIR" -name 'test_*.py' -not -path '*/.venv/*' | sort)
  if [[ ${#TEST_FILES[@]} -eq 0 ]]; then
      log_error "No test_*.py files found under $SCRIPT_DIR"
      FAIL=1
  elif ! (cd "$SCRIPT_DIR" && uv run pytest --collect-only -q >/dev/null 2>&1); then
      log_error "pytest could not collect the test suite"
      FAIL=1
  else
      COLLECTED="$(cd "$SCRIPT_DIR" && uv run pytest --collect-only -q 2>/dev/null \
          | grep -oE '[A-Za-z0-9_/.-]*test_[A-Za-z0-9_]*\.py' | sort -u || true)"
      for f in "${TEST_FILES[@]}"; do
          base="$(basename "$f")"
          if ! grep -qE "(^|/)${base}(::|$)" <<<"$COLLECTED"; then
              log_error "Test file exists but pytest does not collect it: $f"
              FAIL=1
          fi
      done
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
log_info "Running test suite (${#TEST_FILES[@]} files)..."
echo ""

  set +e
  (cd "$SCRIPT_DIR" && uv run pytest -v "${PYTEST_ARGS[@]}")
EXIT_CODE=$?
set -e

echo ""
if [[ $EXIT_CODE -eq 0 ]]; then
    log_ok "All tests passed"
else
    log_error "Some tests failed (exit code: $EXIT_CODE)"
fi

# ---------------------------------------------------------------------------
# Clean up ephemeral databases left by tests
# ---------------------------------------------------------------------------
log_info "Cleaning up ephemeral databases..."
rm -rf /tmp/pytest-zakhor-* /tmp/zakhor-ephemeral-* 2>/dev/null || true
log_ok "Cleanup complete"

exit $EXIT_CODE
