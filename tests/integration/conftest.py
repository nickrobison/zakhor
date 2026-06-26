"""Conftest for integration tests — re-exports fixtures from the main Python conftest.

The main conftest lives at tests/python/conftest.py and provides:

- tracker_available — session-scoped, checks if zakhor binary can start
- zakhor_server — function-scoped, starts zakhor with --http --ephemeral, yields URL
- mcp_session — function-scoped, initialized MCP ClientSession
- sparql_client — module-scoped, SPARQLWrapper for direct SPARQL queries

We inject tests/python/ into sys.path so that ``from conftest import …``
resolves to tests/python/conftest.py.
"""

from __future__ import annotations

import sys
from pathlib import Path

_tests_python = Path(__file__).resolve().parent.parent / "python"
if str(_tests_python) not in sys.path:
    sys.path.insert(0, str(_tests_python))

# fmt: off
from conftest import (  # noqa: E402 — import after sys.path manipulation
    mcp_session,
    sparql_client,
    tracker_available,
    zakhor_server,
)
# fmt: on
