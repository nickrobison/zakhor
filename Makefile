.PHONY: test test-integration build

build:
	cargo build

test:
	cargo test

test-integration:
	cd tests/python && uv pip install -e . && uv run pytest tests/integration/ -v
	$(MAKE) -C . clean-ephemeral-dbs

clean-ephemeral-dbs:
	rm -rf /tmp/zakhor-ephemeral-* 2>/dev/null || true