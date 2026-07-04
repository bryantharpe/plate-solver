#!/usr/bin/env python3
"""Subprocess lifecycle for the eval-harness's Rust gRPC servers.

Owns spawn / health-check / teardown for ``cedar-detect-server`` and
``ps-grpc``. This module only manages the process; adapters.py builds its own
gRPC channel against ``ManagedServer.address`` ("host:port") to make the
actual detect/solve calls.
"""
from __future__ import annotations

import atexit
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Callable, List, Optional

import grpc

ROOT = Path(__file__).resolve().parents[3]
BENCHMARK_DIR = Path(__file__).resolve().parent
GENERATED_DIR = BENCHMARK_DIR / "generated"

PS_GRPC_BIN = ROOT / "target" / "release" / "ps-grpc"
CEDAR_DETECT_BIN = (
    ROOT / "reference-solutions" / "cedar-detect" / "target" / "release" / "cedar-detect-server"
)

HEALTH_CHECK_TIMEOUT_S = 10.0
HEALTH_CHECK_POLL_INTERVAL_S = 0.2


class ServerStartupError(RuntimeError):
    """Raised when a managed server fails to become healthy in time."""


class _OutputCollector:
    """Reads a subprocess's combined stdout+stderr on a background thread into
    a bounded ring buffer, so failures can be diagnosed without blocking on
    the pipe."""

    def __init__(self, proc: subprocess.Popen, max_lines: int = 200):
        self._lines: List[str] = []
        self._max_lines = max_lines
        self._lock = threading.Lock()
        self._thread = threading.Thread(target=self._run, args=(proc,), daemon=True)
        self._thread.start()

    def _run(self, proc: subprocess.Popen) -> None:
        assert proc.stdout is not None
        for line in proc.stdout:
            with self._lock:
                self._lines.append(line.rstrip("\n"))
                if len(self._lines) > self._max_lines:
                    self._lines.pop(0)

    def tail(self, n: int = 40) -> str:
        with self._lock:
            return "\n".join(self._lines[-n:])


class ManagedServer:
    """A spawned gRPC server subprocess with health-check and guaranteed
    teardown (atexit + SIGINT/SIGTERM)."""

    def __init__(self, name: str, argv: List[str]):
        self.name = name
        self._argv = argv
        self._proc: Optional[subprocess.Popen] = None
        self._output: Optional[_OutputCollector] = None
        self.port: Optional[int] = None
        self.address: Optional[str] = None

    def start(self, port: int, health_check: Callable[[], None]) -> "ManagedServer":
        """Spawn the process and block until ``health_check()`` succeeds or
        ``HEALTH_CHECK_TIMEOUT_S`` elapses. ``health_check`` takes no
        arguments, raises on failure, and returns normally on success (e.g. a
        closure making one gRPC call with a short per-call timeout)."""
        self.port = port
        self.address = f"127.0.0.1:{port}"
        self._proc = subprocess.Popen(
            self._argv,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        self._output = _OutputCollector(self._proc)
        _register(self)

        deadline = time.monotonic() + HEALTH_CHECK_TIMEOUT_S
        last_err: Optional[Exception] = None
        while time.monotonic() < deadline:
            if self._proc.poll() is not None:
                raise ServerStartupError(
                    f"{self.name} exited early with code {self._proc.returncode} "
                    f"before health check succeeded:\n{self._output.tail()}"
                )
            try:
                health_check()
                return self
            except Exception as exc:  # noqa: BLE001 - broad on purpose, retried
                last_err = exc
                time.sleep(HEALTH_CHECK_POLL_INTERVAL_S)
        self.stop()
        raise ServerStartupError(
            f"{self.name} did not become healthy within {HEALTH_CHECK_TIMEOUT_S}s "
            f"(last error: {last_err}):\n{self._output.tail() if self._output else ''}"
        )

    def stop(self) -> None:
        if self._proc is None or self._proc.poll() is not None:
            _unregister(self)
            return
        self._proc.terminate()
        try:
            self._proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self._proc.kill()
            self._proc.wait(timeout=5)
        _unregister(self)

    def __enter__(self) -> "ManagedServer":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.stop()


# Global registry so atexit/signal handlers can clean up every server started
# in this process, including ones a caller forgot to stop().
_active: List[ManagedServer] = []
_active_lock = threading.Lock()


def _register(server: ManagedServer) -> None:
    with _active_lock:
        _active.append(server)


def _unregister(server: ManagedServer) -> None:
    with _active_lock:
        if server in _active:
            _active.remove(server)


def _cleanup_all() -> None:
    with _active_lock:
        servers = list(_active)
    for server in servers:
        try:
            server.stop()
        except Exception:  # noqa: BLE001 - best-effort cleanup on exit
            pass


atexit.register(_cleanup_all)


def _signal_handler(signum, frame):
    _cleanup_all()
    signal.signal(signum, signal.SIG_DFL)
    signal.raise_signal(signum)


for _sig in (signal.SIGINT, signal.SIGTERM):
    signal.signal(_sig, _signal_handler)


def _ensure_generated_on_path() -> None:
    generated_str = str(GENERATED_DIR)
    if generated_str not in sys.path:
        sys.path.insert(0, generated_str)


def _ps_grpc_health_check(address: str) -> None:
    _ensure_generated_on_path()
    import plate_solver_pb2  # type: ignore
    import plate_solver_pb2_grpc  # type: ignore

    channel = grpc.insecure_channel(address)
    stub = plate_solver_pb2_grpc.PlateSolverStub(channel)
    stub.GetInfo(plate_solver_pb2.InfoRequest(), timeout=2.0)


def _cedar_detect_health_check(address: str) -> None:
    from tetra3 import cedar_detect_pb2  # type: ignore
    from tetra3 import cedar_detect_pb2_grpc  # type: ignore

    channel = grpc.insecure_channel(address)
    stub = cedar_detect_pb2_grpc.CedarDetectStub(channel)
    tiny_image = cedar_detect_pb2.Image(width=4, height=4, image_data=bytes(16))
    req = cedar_detect_pb2.CentroidsRequest(
        input_image=tiny_image,
        sigma=8.0,
        return_binned=False,
        detect_hot_pixels=True,
        normalize_rows=False,
    )
    stub.ExtractCentroids(req, timeout=2.0)


def start_ps_grpc_server(port: int, db_path: Path) -> ManagedServer:
    """Spawn ``ps-grpc --address 127.0.0.1:<port> --db-path <db_path>`` and
    block until a ``GetInfo`` call succeeds."""
    if not PS_GRPC_BIN.is_file():
        raise ServerStartupError(
            f"ps-grpc binary not found at {PS_GRPC_BIN}; build it with "
            f"`cargo build --release -p ps-grpc`"
        )
    server = ManagedServer(
        "ps-grpc",
        [str(PS_GRPC_BIN), "--address", f"127.0.0.1:{port}", "--db-path", str(db_path)],
    )
    return server.start(port, lambda: _ps_grpc_health_check(server.address))


def start_cedar_detect_server(port: int) -> ManagedServer:
    """Spawn ``cedar-detect-server --port <port>`` and block until a tiny
    ``ExtractCentroids`` call succeeds."""
    if not CEDAR_DETECT_BIN.is_file():
        raise ServerStartupError(
            f"cedar-detect-server binary not found at {CEDAR_DETECT_BIN}; build it with "
            f"`cargo build --release --manifest-path reference-solutions/cedar-detect/Cargo.toml "
            f"--bin cedar-detect-server`"
        )
    server = ManagedServer("cedar-detect-server", [str(CEDAR_DETECT_BIN), "--port", str(port)])
    return server.start(port, lambda: _cedar_detect_health_check(server.address))
