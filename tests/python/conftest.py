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
ZAKHOR_BINARY = PROJECT_ROOT / "target" / "debug" / "zakhor"
SERVER_START_TIMEOUT = 10.0  # seconds to wait for server readiness
POLL_INTERVAL = 0.1  # seconds between readiness checks

# SPARQL endpoint for direct queries (e.g., tracker3 endpoint or tinysparql)
SPARQL_ENDPOINT = os.environ.get("ZAKHOR_SPARQL_ENDPOINT", "http://localhost:7200")

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _find_free_port() -> int:
    """Return a free TCP port on 127.0.0.1."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


async def _is_server_ready(url: str) -> bool:
    """Return True if the MCP server responds to a POST."""
    try:
        async with httpx.AsyncClient(
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
            },
            timeout=httpx.Timeout(5.0),
        ) as client:
            resp = await client.post(
                url,
                json={"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
            )
            # Any HTTP response (2xx, 4xx) means the server is running
            return True
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

    The endpoint URL defaults to http://localhost:7200 and can be overridden
    via the ``ZAKHOR_SPARQL_ENDPOINT`` environment variable.

    Use this fixture when tests need to verify data directly in the Tracker
    SPARQL store (e.g., assert triples were written correctly).
    """
    client = SPARQLWrapper(SPARQL_ENDPOINT)
    client.setReturnFormat(JSON)
    # Use POST for query execution to avoid URL-length limits
    client.setMethod("POST")
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
        proc = await asyncio.create_subprocess_exec(
            str(ZAKHOR_BINARY),
            "--http",
            "--ephemeral",
            f"--db-path={db_path}",
            env=env,
            stdout=asyncio.subprocess.DEVNULL,
            stderr=asyncio.subprocess.DEVNULL,
        )

        url = f"http://127.0.0.1:{port}/"
        for _ in range(int(SERVER_START_TIMEOUT / POLL_INTERVAL)):
            await asyncio.sleep(POLL_INTERVAL)
            if await _is_server_ready(url):
                proc.terminate()
                try:
                    await asyncio.wait_for(proc.wait(), timeout=5)
                except (asyncio.TimeoutError, ProcessLookupError):
                    pass
                _cleanup_db(db_path)
                return True

        proc.terminate()
        try:
            await asyncio.wait_for(proc.wait(), timeout=5)
        except (asyncio.TimeoutError, ProcessLookupError):
            pass
        _cleanup_db(db_path)
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
    url = f"http://127.0.0.1:{port}/"

    env = os.environ.copy()
    env["ZAKHOR_HTTP_PORT"] = str(port)
    env["ZAKHOR_HTTP_HOST"] = "127.0.0.1"

    proc = await asyncio.create_subprocess_exec(
        str(ZAKHOR_BINARY),
        "--http",
        "--ephemeral",
        f"--db-path={db_path}",
        env=env,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
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
        pytest.fail(f"zakhor server failed to start at {url}")

    logger.info("zakhor server started at %s (pid=%d)", url, proc.pid)

    try:
        yield url
    finally:
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

        # Read and discard remaining stdout/stderr
        for stream_name in ("stdout", "stderr"):
            stream = getattr(proc, stream_name)
            if stream and not stream.at_eof():
                try:
                    await asyncio.wait_for(stream.read(), timeout=1)
                except (asyncio.TimeoutError, OSError):
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
        await task
