#!/usr/bin/env python3
"""Uniform detect() / solve_from_image() interface over the three plate-solving
systems compared by the eval-harness.

Every adapter exposes the same two methods with the same call signature and
returns the same result-dict shape, so ``run_benchmark.py`` can drive all
three identically:

- ``TetraOriginalAdapter``: spawns ``tetra3_original_runner.py`` as a
  subprocess under ``tools/parity/.venv-tetra3-orig`` (original tetra3 cannot
  share a Python process with cedar-solve - both install a top-level
  ``tetra3`` package; see ``openspec/changes/feat-09-eval-harness/design.md``).
- ``CedarFlowAdapter``: gRPC ``ExtractCentroids`` against a running
  ``cedar-detect-server``, then in-process cedar-solve ``solve_from_centroids``.
- ``PsGrpcAdapter``: gRPC ``ExtractCentroids``/``SolveFromCentroids`` (detect
  and centroids-only calls) plus ``ExtractCentroids``/``SolveFromCentroids``
  composed together for the full solve, against a running ``ps-grpc`` server.

Explicit shared detection parameters (task 2.4) are the defaults for every
adapter call: ``sigma=4.0`` (matches the value ``ps_solve::solve_from_image``
is hard-coded to - see design.md's C2 note), ``detect_hot_pixels=True``,
``normalize_rows=False``, no binning.

Dual timing capture (task 2.5): every result carries both client wall-clock
(``time.perf_counter()``) and each system's self-reported algorithm-only
time. ``ps-grpc``'s ``Solution.t_extract_ms`` is hard-coded to 0.0 in
``ps-grpc/src/service.rs`` (SolveFromImage and SolveFromCentroids both), so
``PsGrpcAdapter.solve_from_image`` always sources extraction timing from its
own ``ExtractCentroids`` call rather than ``Solution.t_extract_ms``.
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional

import grpc
import numpy as np

BENCHMARK_DIR = Path(__file__).resolve().parent
GENERATED_DIR = BENCHMARK_DIR / "generated"

# Every gRPC call in this module passes an explicit deadline - without one, a
# hung/deadlocked server (e.g. on one of the corpus's two stress images,
# included specifically to probe pathological input) would block the whole
# benchmark run forever instead of surfacing as a per-image error.
GRPC_CALL_TIMEOUT_S = 10.0

# cedar-solve's tetra3.py defines these as module-level int constants (not
# strings) - see reference-solutions/cedar-solve/tetra3/tetra3.py lines
# 139-143. Mapped here so CedarFlowAdapter's "status" field matches the
# string enum names PsGrpcAdapter/TetraOriginalAdapter both report.
_CEDAR_SOLVE_STATUS_NAMES = {
    1: "MATCH_FOUND",
    2: "NO_MATCH",
    3: "TIMEOUT",
    4: "CANCELLED",
    5: "TOO_FEW",
}

# Task 2.4: explicit shared detection parameters, threaded through every
# adapter call instead of left at each system's divergent defaults.
SHARED_PARAMS: Dict[str, Any] = {
    "sigma": 4.0,
    "detect_hot_pixels": True,
    "normalize_rows": False,
    "binning": None,
    "match_radius": 0.01,
    "match_threshold": 1e-5,
    "distortion": 0.0,
}


def _ensure_generated_on_path() -> None:
    generated_str = str(GENERATED_DIR)
    if generated_str not in sys.path:
        sys.path.insert(0, generated_str)


def _load_image_grayscale(image_path: Path):
    """Returns (width, height, row-major uint8 grayscale bytes)."""
    from PIL import Image

    with Image.open(image_path) as img:
        gray = img.convert("L")
        width, height = gray.size
        image_data = gray.tobytes()
    return width, height, image_data


def _build_centroids_request(pb2_module, width, height, image_data, sigma, detect_hot_pixels,
                              normalize_rows, binning):
    """Builds a CentroidsRequest against whichever proto module is passed in -
    ``plate_solver_pb2`` and cedar-solve's ``cedar_detect_pb2`` declare
    identically-shaped ``Image``/``CentroidsRequest`` messages."""
    image_msg = pb2_module.Image(width=width, height=height, image_data=image_data)
    kwargs = dict(
        input_image=image_msg,
        sigma=sigma,
        return_binned=False,
        detect_hot_pixels=detect_hot_pixels,
        normalize_rows=normalize_rows,
    )
    if binning is not None:
        kwargs["binning"] = binning
    return pb2_module.CentroidsRequest(**kwargs)


def _check_iterations(n_iterations: int, warmup: int) -> None:
    if n_iterations < 1:
        raise ValueError(f"n_iterations must be >= 1, got {n_iterations}")
    if warmup < 0:
        raise ValueError(f"warmup must be >= 0, got {warmup}")


def _coerce_matched_cat_ids(raw_ids) -> Optional[List[Any]]:
    """cedar-solve's/tetra3's ``matched_catID`` is a flat array of scalar IDs
    when the loaded catalog's ``star_catalog_IDs`` is 1-D, but can be a
    per-star tuple of catalog-specific columns when it's 2-D (see
    reference-solutions/cedar-solve/tetra3/tetra3.py's ``star_catalog_IDs``
    handling). Preserve whichever shape it actually is instead of assuming
    scalar and crashing on ``int(list)``."""
    if raw_ids is None:
        return None
    try:
        return [int(x) for x in raw_ids]
    except TypeError:
        return [list(x) for x in raw_ids]


def _make_detect_result(*, system: str, image_name: str, n_iterations: int, warmup: int,
                         wall_clock_s: List[float], algorithm_s: List[Optional[float]],
                         noise_estimate: Optional[float], hot_pixel_count: Optional[int],
                         centroids_yx: List[tuple]) -> Dict[str, Any]:
    return {
        "system": system,
        "image_name": image_name,
        "n_iterations": n_iterations,
        "warmup": warmup,
        "wall_clock_s": wall_clock_s,
        "algorithm_s": algorithm_s,
        "noise_estimate": noise_estimate,
        "hot_pixel_count": hot_pixel_count,
        "centroids_yx": [[float(y), float(x)] for (y, x) in centroids_yx],
    }


def _make_solve_result(*, system: str, image_name: str, catalog: str, n_iterations: int, warmup: int,
                        wall_clock_s: List[float], t_extract_ms: List[Optional[float]],
                        t_solve_ms: List[Optional[float]], status: Optional[str],
                        ra_deg: Optional[float], dec_deg: Optional[float], roll_deg: Optional[float],
                        fov_deg: Optional[float], matches: Optional[int],
                        matched_cat_ids: Optional[List[Any]], rmse_arcsec: Optional[float],
                        p90e_arcsec: Optional[float], maxe_arcsec: Optional[float]) -> Dict[str, Any]:
    return {
        "system": system,
        "image_name": image_name,
        "catalog": catalog,
        "n_iterations": n_iterations,
        "warmup": warmup,
        "wall_clock_s": wall_clock_s,
        "t_extract_ms": t_extract_ms,
        "t_solve_ms": t_solve_ms,
        "status": status,
        "ra_deg": ra_deg,
        "dec_deg": dec_deg,
        "roll_deg": roll_deg,
        "fov_deg": fov_deg,
        "matches": matches,
        "matched_cat_ids": matched_cat_ids,
        "rmse_arcsec": rmse_arcsec,
        "p90e_arcsec": p90e_arcsec,
        "maxe_arcsec": maxe_arcsec,
    }


class _GrpcExtractMixin:
    """Shared ``ExtractCentroids``-over-gRPC plumbing for the two adapters
    that measure a Rust server this way (``PsGrpcAdapter``, ``CedarFlowAdapter``).
    Subclasses set ``self._pb2`` (the generated proto module) and
    ``self._stub`` (a stub with an ``ExtractCentroids`` method) in ``__init__``."""

    def _extract_once(self, width, height, image_data, sigma, detect_hot_pixels, normalize_rows, binning):
        req = _build_centroids_request(
            self._pb2, width, height, image_data, sigma, detect_hot_pixels, normalize_rows, binning
        )
        t0 = time.perf_counter()
        resp = self._stub.ExtractCentroids(req, timeout=GRPC_CALL_TIMEOUT_S)
        wall = time.perf_counter() - t0
        algo_s = resp.algorithm_time.ToTimedelta().total_seconds()
        centroids_yx = [(c.centroid_position.y, c.centroid_position.x) for c in resp.star_candidates]
        return wall, algo_s, centroids_yx, resp.noise_estimate, resp.hot_pixel_count

    def detect(self, image_path: Path, n_iterations: int, warmup: int,
               sigma: float = SHARED_PARAMS["sigma"],
               detect_hot_pixels: bool = SHARED_PARAMS["detect_hot_pixels"],
               normalize_rows: bool = SHARED_PARAMS["normalize_rows"],
               binning: Optional[int] = SHARED_PARAMS["binning"]) -> Dict[str, Any]:
        _check_iterations(n_iterations, warmup)
        width, height, image_data = _load_image_grayscale(image_path)
        wall_times: List[float] = []
        algo_times: List[Optional[float]] = []
        centroids_yx: List[tuple] = []
        noise_estimate = hot_pixel_count = None
        for i in range(warmup + n_iterations):
            wall, algo_s, centroids_yx, noise_estimate, hot_pixel_count = self._extract_once(
                width, height, image_data, sigma, detect_hot_pixels, normalize_rows, binning
            )
            if i >= warmup:
                wall_times.append(wall)
                algo_times.append(algo_s)
        return _make_detect_result(
            system=self.system, image_name=image_path.name, n_iterations=n_iterations, warmup=warmup,
            wall_clock_s=wall_times, algorithm_s=algo_times,
            noise_estimate=noise_estimate, hot_pixel_count=hot_pixel_count, centroids_yx=centroids_yx,
        )


class PsGrpcAdapter(_GrpcExtractMixin):
    """Measures the new Rust workflow via ``ps-grpc``'s gRPC interface.

    ``solve_from_image`` composes ``ExtractCentroids`` + ``SolveFromCentroids``
    (per spec.md's "New workflow measured via its own gRPC server" scenario,
    which names all three RPCs) rather than the single fused ``SolveFromImage``
    call: ``Solution.t_extract_ms`` is hard-coded 0.0 on BOTH RPCs
    (ps-grpc/src/service.rs), so a standalone ``ExtractCentroids`` call is the
    only source of a real self-reported extraction time either way - composing
    it this way gets that timing from the same call that also produces the
    centroids passed to ``SolveFromCentroids``, instead of paying for a second,
    redundant extraction just to recover the timing. Wall-clock covers both
    RPCs combined, matching how ``CedarFlowAdapter`` also measures extract+solve
    as one combined wall-clock span.
    """

    system = "ps_grpc"
    catalog = "shared_cedar_solve"

    def __init__(self, address: str):
        _ensure_generated_on_path()
        import plate_solver_pb2 as pb2  # type: ignore
        import plate_solver_pb2_grpc as pb2_grpc  # type: ignore

        self._pb2 = pb2
        self._channel = grpc.insecure_channel(address)
        self._stub = pb2_grpc.PlateSolverStub(self._channel)

    def solve_from_image(self, image_path: Path, n_iterations: int, warmup: int,
                          fov_estimate: Optional[float] = None, fov_max_error: Optional[float] = None,
                          timeout_s: float = 5.0,
                          sigma: float = SHARED_PARAMS["sigma"],
                          detect_hot_pixels: bool = SHARED_PARAMS["detect_hot_pixels"],
                          normalize_rows: bool = SHARED_PARAMS["normalize_rows"],
                          binning: Optional[int] = SHARED_PARAMS["binning"],
                          match_radius: float = SHARED_PARAMS["match_radius"],
                          match_threshold: float = SHARED_PARAMS["match_threshold"],
                          distortion: float = SHARED_PARAMS["distortion"]) -> Dict[str, Any]:
        _check_iterations(n_iterations, warmup)
        width, height, image_data = _load_image_grayscale(image_path)
        wall_times: List[float] = []
        t_extract_list: List[Optional[float]] = []
        t_solve_list: List[Optional[float]] = []
        last = None
        for i in range(warmup + n_iterations):
            t0 = time.perf_counter()
            extract_req = _build_centroids_request(
                self._pb2, width, height, image_data, sigma, detect_hot_pixels, normalize_rows, binning
            )
            extract_resp = self._stub.ExtractCentroids(extract_req, timeout=GRPC_CALL_TIMEOUT_S)
            extract_algo_s = extract_resp.algorithm_time.ToTimedelta().total_seconds()
            centroids = [
                self._pb2.ImageCoord(x=c.centroid_position.x, y=c.centroid_position.y)
                for c in extract_resp.star_candidates
            ]

            params_kwargs = dict(
                match_radius=match_radius,
                match_threshold=match_threshold,
                solve_timeout_ms=int(round(timeout_s * 1000)),
                distortion=distortion,
                return_matches=True,
            )
            if fov_estimate is not None:
                params_kwargs["fov_estimate"] = fov_estimate
            if fov_max_error is not None:
                params_kwargs["fov_max_error"] = fov_max_error
            solve_req = self._pb2.SolveFromCentroidsRequest(
                centroids=centroids, width=width, height=height,
                params=self._pb2.SolveParams(**params_kwargs),
            )
            solution = self._stub.SolveFromCentroids(solve_req, timeout=timeout_s + GRPC_CALL_TIMEOUT_S)
            wall = time.perf_counter() - t0

            assert solution.t_extract_ms == 0.0, (
                "ps-grpc's Solution.t_extract_ms is no longer hard-coded to 0.0 - "
                "the standalone ExtractCentroids timing workaround in this adapter "
                "can be removed (see design.md's C2-adjacent note)."
            )
            if i >= warmup:
                wall_times.append(wall)
                t_extract_list.append(extract_algo_s * 1000.0)
                t_solve_list.append(solution.t_solve_ms)
            last = solution

        status_name = self._pb2.SolveStatus.Name(last.status)
        if status_name == "MATCH_FOUND":
            ra_deg, dec_deg, roll_deg, fov_deg = last.ra, last.dec, last.roll, last.fov
            matches = last.matches
            matched_cat_ids = [m.cat_id for m in last.matched]
            rmse_arcsec, p90e_arcsec, maxe_arcsec = last.rmse, last.p90e, last.maxe
        else:
            # proto3 scalar fields default to 0.0/0 rather than being unset;
            # normalize to None on failure so this matches CedarFlowAdapter's
            # and TetraOriginalAdapter's None-on-no-solution convention
            # (the whole point of a "uniform" result schema).
            ra_deg = dec_deg = roll_deg = fov_deg = None
            matches = None
            matched_cat_ids = None
            rmse_arcsec = p90e_arcsec = maxe_arcsec = None
        return _make_solve_result(
            system=self.system, image_name=image_path.name, catalog=self.catalog,
            n_iterations=n_iterations, warmup=warmup,
            wall_clock_s=wall_times, t_extract_ms=t_extract_list, t_solve_ms=t_solve_list,
            status=status_name, ra_deg=ra_deg, dec_deg=dec_deg, roll_deg=roll_deg, fov_deg=fov_deg,
            matches=matches, matched_cat_ids=matched_cat_ids,
            rmse_arcsec=rmse_arcsec, p90e_arcsec=p90e_arcsec, maxe_arcsec=maxe_arcsec,
        )


class CedarFlowAdapter(_GrpcExtractMixin):
    """Measures cedar-detect (gRPC) + in-process cedar-solve."""

    system = "cedar_flow"
    catalog = "shared_cedar_solve"

    def __init__(self, cedar_detect_address: str, catalog_path: Path):
        from tetra3 import cedar_detect_pb2 as pb2  # type: ignore
        from tetra3 import cedar_detect_pb2_grpc as pb2_grpc  # type: ignore
        import tetra3  # type: ignore

        self._pb2 = pb2
        self._channel = grpc.insecure_channel(cedar_detect_address)
        self._stub = pb2_grpc.CedarDetectStub(self._channel)
        self._t3 = tetra3.Tetra3(load_database=None)
        self._t3.load_database(catalog_path)

    def solve_from_image(self, image_path: Path, n_iterations: int, warmup: int,
                          fov_estimate: Optional[float] = None, fov_max_error: Optional[float] = None,
                          timeout_s: float = 5.0,
                          sigma: float = SHARED_PARAMS["sigma"],
                          detect_hot_pixels: bool = SHARED_PARAMS["detect_hot_pixels"],
                          normalize_rows: bool = SHARED_PARAMS["normalize_rows"],
                          binning: Optional[int] = SHARED_PARAMS["binning"],
                          match_radius: float = SHARED_PARAMS["match_radius"],
                          match_threshold: float = SHARED_PARAMS["match_threshold"],
                          distortion: float = SHARED_PARAMS["distortion"]) -> Dict[str, Any]:
        _check_iterations(n_iterations, warmup)
        width, height, image_data = _load_image_grayscale(image_path)
        wall_times: List[float] = []
        t_extract_list: List[Optional[float]] = []
        t_solve_list: List[Optional[float]] = []
        last: Dict[str, Any] = {}
        for i in range(warmup + n_iterations):
            t0 = time.perf_counter()
            _, extract_algo_s, centroids_yx, _, _ = self._extract_once(
                width, height, image_data, sigma, detect_hot_pixels, normalize_rows, binning
            )
            centroids_arr = (
                np.array(centroids_yx, dtype=np.float64) if centroids_yx else np.zeros((0, 2))
            )
            sol = self._t3.solve_from_centroids(
                centroids_arr,
                size=(height, width),
                fov_estimate=fov_estimate,
                fov_max_error=fov_max_error,
                match_radius=match_radius,
                match_threshold=match_threshold,
                solve_timeout=timeout_s * 1000,
                distortion=distortion,
                return_matches=True,
            )
            wall = time.perf_counter() - t0

            if i >= warmup:
                wall_times.append(wall)
                t_extract_list.append(extract_algo_s * 1000.0)
                t_solve_list.append(sol.get("T_solve"))
            last = sol

        # cedar-solve's own status field is an int (MATCH_FOUND=1, NO_MATCH=2,
        # TIMEOUT=3, CANCELLED=4, TOO_FEW=5 - tetra3.py lines 139-143), not a
        # string; normalize it so this matches PsGrpcAdapter's/
        # TetraOriginalAdapter's string-enum-name convention.
        status_code = last.get("status")
        status_name = _CEDAR_SOLVE_STATUS_NAMES.get(status_code, str(status_code)) if status_code is not None else None
        matched_cat_ids = _coerce_matched_cat_ids(last.get("matched_catID"))
        return _make_solve_result(
            system=self.system, image_name=image_path.name, catalog=self.catalog,
            n_iterations=n_iterations, warmup=warmup,
            wall_clock_s=wall_times, t_extract_ms=t_extract_list, t_solve_ms=t_solve_list,
            status=status_name,
            ra_deg=last.get("RA"), dec_deg=last.get("Dec"), roll_deg=last.get("Roll"), fov_deg=last.get("FOV"),
            matches=last.get("Matches"), matched_cat_ids=matched_cat_ids,
            rmse_arcsec=last.get("RMSE"), p90e_arcsec=last.get("P90E"), maxe_arcsec=last.get("MAXE"),
        )


class TetraOriginalAdapter:
    """Runs original tetra3 as a subprocess under its own venv
    (``tools/parity/.venv-tetra3-orig``), since it cannot share a Python
    process with cedar-solve (both install a top-level ``tetra3`` package).

    Batches all N (+ warmup) iterations into ONE subprocess invocation of
    ``tetra3_original_runner.py``, so interpreter/import/database-load
    startup cost stays out of the timed region - matching how the two Rust
    servers are also long-lived, not re-spawned per call.
    """

    system = "tetra3_original"
    catalog = "tetra3_original_bundled"

    def __init__(self, venv_python: Path, db_path: Path):
        self._venv_python = Path(venv_python)
        self._db_path = Path(db_path)
        self._runner_script = BENCHMARK_DIR / "tetra3_original_runner.py"

    def _run_subprocess(self, mode: str, image_path: Path, n_iterations: int, warmup: int,
                         sigma: float, fov_estimate: Optional[float], fov_max_error: Optional[float],
                         match_radius: float, match_threshold: float, distortion: float,
                         timeout_s: float) -> Dict[str, Any]:
        argv = [
            str(self._venv_python), str(self._runner_script),
            "--mode", mode,
            "--image", str(image_path),
            "--db-path", str(self._db_path),
            "--sigma", str(sigma),
            "--n-iterations", str(n_iterations),
            "--warmup", str(warmup),
            "--match-radius", str(match_radius),
            "--match-threshold", str(match_threshold),
            "--distortion", str(distortion),
            "--timeout-ms", str(int(timeout_s * 1000)),
        ]
        if fov_estimate is not None:
            argv += ["--fov-estimate", str(fov_estimate)]
        if fov_max_error is not None:
            argv += ["--fov-max-error", str(fov_max_error)]

        result = subprocess.run(argv, capture_output=True, text=True)
        if result.returncode != 0:
            raise RuntimeError(
                f"tetra3_original_runner.py ({mode}) failed for {image_path.name} "
                f"with code {result.returncode}:\nstdout: {result.stdout}\nstderr: {result.stderr}"
            )
        return json.loads(result.stdout)

    def detect(self, image_path: Path, n_iterations: int, warmup: int,
               sigma: float = SHARED_PARAMS["sigma"],
               detect_hot_pixels: bool = SHARED_PARAMS["detect_hot_pixels"],
               normalize_rows: bool = SHARED_PARAMS["normalize_rows"],
               binning: Optional[int] = SHARED_PARAMS["binning"]) -> Dict[str, Any]:
        # detect_hot_pixels/normalize_rows/binning are cedar-detect-specific
        # knobs with no equivalent in original tetra3's centroid extractor;
        # only sigma is a truly shared parameter here (see design.md).
        _check_iterations(n_iterations, warmup)
        payload = self._run_subprocess(
            "detect", image_path, n_iterations, warmup, sigma,
            fov_estimate=None, fov_max_error=None,
            match_radius=SHARED_PARAMS["match_radius"],
            match_threshold=SHARED_PARAMS["match_threshold"],
            distortion=SHARED_PARAMS["distortion"], timeout_s=5.0,
        )
        return _make_detect_result(
            system=self.system, image_name=image_path.name, n_iterations=n_iterations, warmup=warmup,
            wall_clock_s=payload["wall_clock_s"], algorithm_s=payload["algorithm_s"],
            noise_estimate=payload.get("noise_estimate"), hot_pixel_count=payload.get("hot_pixel_count"),
            centroids_yx=payload.get("centroids_yx", []),
        )

    def solve_from_image(self, image_path: Path, n_iterations: int, warmup: int,
                          fov_estimate: Optional[float] = None, fov_max_error: Optional[float] = None,
                          timeout_s: float = 5.0,
                          sigma: float = SHARED_PARAMS["sigma"],
                          detect_hot_pixels: bool = SHARED_PARAMS["detect_hot_pixels"],
                          normalize_rows: bool = SHARED_PARAMS["normalize_rows"],
                          binning: Optional[int] = SHARED_PARAMS["binning"],
                          match_radius: float = SHARED_PARAMS["match_radius"],
                          match_threshold: float = SHARED_PARAMS["match_threshold"],
                          distortion: float = SHARED_PARAMS["distortion"]) -> Dict[str, Any]:
        _check_iterations(n_iterations, warmup)
        payload = self._run_subprocess(
            "solve", image_path, n_iterations, warmup, sigma,
            fov_estimate, fov_max_error, match_radius, match_threshold, distortion, timeout_s,
        )
        return _make_solve_result(
            system=self.system, image_name=image_path.name, catalog=self.catalog,
            n_iterations=n_iterations, warmup=warmup,
            wall_clock_s=payload["wall_clock_s"], t_extract_ms=payload["t_extract_ms"],
            t_solve_ms=payload["t_solve_ms"], status=payload["status"],
            ra_deg=payload.get("ra_deg"), dec_deg=payload.get("dec_deg"),
            roll_deg=payload.get("roll_deg"), fov_deg=payload.get("fov_deg"),
            matches=payload.get("matches"), matched_cat_ids=payload.get("matched_cat_ids"),
            rmse_arcsec=payload.get("rmse_arcsec"), p90e_arcsec=payload.get("p90e_arcsec"),
            maxe_arcsec=payload.get("maxe_arcsec"),
        )
