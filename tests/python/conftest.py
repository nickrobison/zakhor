"""Pytest fixtures for Zakhor MCP integration tests.

Fixtures:
    tracker_available -- session-scoped, checks if zakhor binary can start
    zakhor_server -- function-scoped, starts zakhor with --http --ephemeral, yields URL
    mcp_session -- function-scoped, returns initialized MCP ClientSession
    sparql_client -- module-scoped, returns SPARQLWrapper for direct SPARQL queries
"""

from __future__ import annotations

import asyncio
import logging
import os
import platform
import signal
import socket
from pathlib import Path
from typing import AsyncIterator

import httpx
import pytest
import pytest_asyncio
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client
from SPARQLWrapper import JSON, SPARQLWrapper

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
# Honour CARGO_TARGET_DIR so a customised target directory does not look like a
# missing build; cargo may put the debug binary somewhere other than
# <workspace>/target.
_TARGET_DIR = Path(os.environ.get("CARGO_TARGET_DIR") or (PROJECT_ROOT / "target"))
ZAKHOR_BINARY = _TARGET_DIR / "debug" / "zakhor"
SERVER_START_TIMEOUT = 10.0  # seconds to wait for server readiness
POLL_INTERVAL = 0.1  # seconds between readiness checks

# SPARQL endpoint for direct queries (e.g., tracker3 endpoint or tinysparql).
# Not set by default; tests that need direct SPARQL are skipped unless it is.
SPARQL_ENDPOINT = os.environ.get("ZAKHOR_SPARQL_ENDPOINT", "")

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _find_free_port() -> int:
    """Return a free TCP port on 127.0.0.1."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _open_server_log(tmp_dir: Path | None = None):
    """Create a log file for a server subprocess.

    Returns ``(fileobj, path)``. The caller passes ``fileobj`` as the
    subprocess stdout/stderr and must close it after the server exits;
    ``path`` is where the output landed for post-mortem diagnostics.
    """
    import tempfile

    kwargs = {
        "prefix": "zakhor-server-",
        "suffix": ".log",
        "mode": "w+b",
        "delete": False,
    }
    if tmp_dir is not None:
        kwargs["dir"] = tmp_dir
    tmp = tempfile.NamedTemporaryFile(**kwargs)
    return tmp, Path(tmp.name)


def _tail(path: Path, max_bytes: int = 8192) -> str:
    """Return the last ``max_bytes`` of a log file, or a placeholder if missing."""
    try:
        size = path.stat().st_size
        with open(path, "rb") as fh:
            if size > max_bytes:
                fh.seek(-max_bytes, 2)
            return fh.read().decode("utf-8", errors="replace")
    except OSError:
        return "<server log unavailable>"


def _health_url(mcp_url: str) -> str:
    """Derive the /api/v1/health URL from an MCP /mcp URL."""
    assert mcp_url.endswith("/mcp")
    return mcp_url[: -len("/mcp")] + "/api/v1/health"


async def _is_server_ready(url: str) -> bool:
    """Return True if the server reports a healthy Tracker connection.

    Uses the /api/v1/health endpoint, which returns 200 only when the server
    is up and its Tracker store answers an ASK query. Anything else (connection
    error, non-200) means "not ready".
    """
    try:
        async with httpx.AsyncClient(timeout=httpx.Timeout(5.0)) as client:
            resp = await client.get(_health_url(url))
            return resp.status_code == 200
    except (
        httpx.ConnectError,
        httpx.RemoteProtocolError,
        httpx.ReadTimeout,
        httpx.TimeoutException,
    ):
        return False


# ---------------------------------------------------------------------------
# Module-scoped: SPARQL client for direct database queries
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def sparql_client() -> SPARQLWrapper:
    """Return a SPARQLWrapper connected to the configured SPARQL endpoint.

    The endpoint URL is taken from the ``ZAKHOR_SPARQL_ENDPOINT`` environment
    variable. Tests using this fixture are skipped when it is not set.

    Uses POSTDIRECT: SPARQLWrapper's form-encoded POST method hangs against
    tinysparql, so the query travels as a raw ``application/sparql-query``
    body instead.
    """
    if not SPARQL_ENDPOINT:
        raise pytest.skip("ZAKHOR_SPARQL_ENDPOINT not set")
    client = SPARQLWrapper(SPARQL_ENDPOINT)
    client.setReturnFormat(JSON)
    client.setMethod("POSTDIRECT")
    client.setTimeout(30)
    return client


# ---------------------------------------------------------------------------
# Session-scoped: check if zakhor binary works
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture(scope="session")
async def tracker_available() -> bool:
    """Check whether the zakhor binary can start (requires Tracker libs).

    Returns True if the compiled binary exists and launches successfully.
    """
    if not ZAKHOR_BINARY.exists():
        logger.warning("zakhor binary not found at %s", ZAKHOR_BINARY)
        return False

    # Quick smoke test: start zakhor on a random port, verify it binds
    port = _find_free_port()
    db_path = f"/tmp/pytest-zakhor-smoke-{port}"
    env = os.environ.copy()
    env["ZAKHOR_HTTP_PORT"] = str(port)

    try:
        log_file, log_path = _open_server_log()
        proc = await asyncio.create_subprocess_exec(
            str(ZAKHOR_BINARY),
            "--http",
            "--ephemeral",
            f"--db-path={db_path}",
            env=env,
            stdout=log_file,
            stderr=asyncio.subprocess.STDOUT,
        )

        url = f"http://127.0.0.1:{port}/mcp"
        for _ in range(int(SERVER_START_TIMEOUT / POLL_INTERVAL)):
            await asyncio.sleep(POLL_INTERVAL)
            if await _is_server_ready(url):
                proc.terminate()
                try:
                    await asyncio.wait_for(proc.wait(), timeout=5)
                except (asyncio.TimeoutError, ProcessLookupError):
                    pass
                log_file.close()
                _cleanup_db(db_path)
                log_path.unlink(missing_ok=True)
                return True

        logger.warning(
            "zakhor binary smoke test failed; server output:\n%s",
            _tail(log_path),
        )
        proc.terminate()
        try:
            await asyncio.wait_for(proc.wait(), timeout=5)
        except (asyncio.TimeoutError, ProcessLookupError):
            pass
        log_file.close()
        _cleanup_db(db_path)
        log_path.unlink(missing_ok=True)
        return False
    except (OSError, asyncio.TimeoutError) as exc:
        logger.warning("Tracker/binary smoke test failed: %s", exc)
        _cleanup_db(db_path)
        return False


def _cleanup_db(db_path: str) -> None:
    """Remove a database directory."""
    import shutil

    p = Path(db_path)
    if p.exists():
        shutil.rmtree(p, ignore_errors=True)


# ---------------------------------------------------------------------------
# Function-scoped: start a fresh zakhor server per test
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def zakhor_server(tmp_path: Path) -> AsyncIterator[str]:
    """Start a zakhor server with a fresh Tracker DB and yield its URL.

    The server is terminated and the temp DB cleaned up after the test.
    """
    db_path = tmp_path / "zakhor-db"
    db_path.mkdir(parents=True, exist_ok=True)
    port = _find_free_port()
    url = f"http://127.0.0.1:{port}/mcp"

    env = os.environ.copy()
    env["ZAKHOR_HTTP_PORT"] = str(port)
    env["ZAKHOR_HTTP_HOST"] = "127.0.0.1"

    log_file, log_path = _open_server_log(tmp_path)
    proc = await asyncio.create_subprocess_exec(
        str(ZAKHOR_BINARY),
        "--http",
        "--ephemeral",
        f"--db-path={db_path}",
        env=env,
        stdout=log_file,
        stderr=asyncio.subprocess.STDOUT,
    )

    # Wait for server to be ready
    try:
        async with asyncio.timeout(SERVER_START_TIMEOUT):
            while True:
                await asyncio.sleep(POLL_INTERVAL)
                if await _is_server_ready(url):
                    break
    except TimeoutError:
        proc.terminate()
        try:
            await asyncio.wait_for(proc.wait(), timeout=5)
        except (asyncio.TimeoutError, ProcessLookupError):
            pass
        log_file.close()
        pytest.fail(
            f"zakhor server failed to start at {url}\nserver output:\n{_tail(log_path)}"
        )

    logger.info("zakhor server started at %s (pid=%d)", url, proc.pid)

    try:
        yield url
    finally:
        log_file.close()
        # Teardown: kill process
        if proc.returncode is None:
            if platform.system() == "Windows":
                proc.terminate()
            else:
                proc.terminate()
            try:
                await asyncio.wait_for(proc.wait(), timeout=5)
            except (asyncio.TimeoutError, ProcessLookupError):
                logger.warning(
                    "zakhor server (pid=%d) did not terminate gracefully", proc.pid
                )
                try:
                    proc.kill()
                    await asyncio.wait_for(proc.wait(), timeout=2)
                except (asyncio.TimeoutError, ProcessLookupError):
                    pass

        logger.info("zakhor server (pid=%d) terminated", proc.pid)


# ---------------------------------------------------------------------------
# Function-scoped: MCP ClientSession connected to zakhor
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def mcp_session(zakhor_server: str) -> AsyncIterator[ClientSession]:
    """Yield an initialized MCP ClientSession connected to the zakhor server.

    The session is automatically initialized and closed.

    The HTTP client and MCP session context managers are entered and exited
    within a single dedicated asyncio Task to satisfy anyio 4.x cancel-scope
    tracking (cancel scopes must be entered and exited in the same task).
    """
    session_ready: asyncio.Event = asyncio.Event()
    test_done: asyncio.Event = asyncio.Event()
    session_holder: list[ClientSession] = []
    error_holder: list[BaseException] = []

    async def _run_session() -> None:
        try:
            async with streamable_http_client(url=zakhor_server) as streams:
                read_stream, write_stream, _ = streams
                async with ClientSession(read_stream, write_stream) as session:
                    await session.initialize()
                    logger.info("MCP session initialized at %s", zakhor_server)
                    session_holder.append(session)
                    session_ready.set()
                    await test_done.wait()
        except (Exception, asyncio.CancelledError) as exc:
            error_holder.append(exc)
            if not session_ready.is_set():
                session_ready.set()

    task = asyncio.create_task(_run_session())
    await session_ready.wait()

    if error_holder:
        test_done.set()
        await task
        raise error_holder[0]

    try:
        yield session_holder[0]
    finally:
        test_done.set()
        try:
            await asyncio.wait_for(task, timeout=10)
        except (asyncio.TimeoutError, asyncio.CancelledError):
            task.cancel()
            try:
                await asyncio.wait_for(task, timeout=2)
            except (asyncio.TimeoutError, asyncio.CancelledError):
                pass
