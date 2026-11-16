# Tsuki (月) Makefile
# Standard development, testing, and deployment commands

.PHONY: all build test test-rust test-python test-safety fmt clippy demo dashboard install clean

all: build test

build:
	cargo build --workspace --release

build-dev:
	cargo build --workspace

test: test-rust test-python test-safety

test-rust:
	cargo test --workspace

test-safety:
	cargo test --test safety_suite

test-python:
	PYTHONPATH=bench:reasoning:training .venv/bin/pytest bench/tests training/tests reasoning/tests

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

eval:
	PYTHONPATH=bench:reasoning:training .venv/bin/python bench/eval_suite.py

demo:
	./demo/run_demo.sh

dashboard:
	.venv/bin/streamlit run reasoning/tsuki_api/dashboard.py

install: build
	install -d /usr/local/bin
	install -m 755 target/release/tsuki-guard /usr/local/bin/tsuki-guard
	install -m 755 target/release/tsuki-actuator /usr/local/bin/tsuki-actuator
	install -m 755 target/release/tsuki-rollbackd /usr/local/bin/tsuki-rollbackd
	install -d /etc/tsuki
	install -m 644 tsuki.toml.example /etc/tsuki/tsuki.toml
	install -d /var/run/tsuki/snapshots
	install -d /var/lib/tsuki
	@echo "[✓] Tsuki binaries installed to /usr/local/bin/"

clean:
	cargo clean
	find . -type d -name "__pycache__" -exec rm -rf {} +
	find . -type d -name ".pytest_cache" -exec rm -rf {} +
