"""
Generate the Python gRPC code for ps-grpc's plate_solver.proto.

Mirrors reference-solutions/cedar-solve/scripts/compile_proto.py's invocation
shape (pathlib-computed proto root / target dir, a virtual -I prefix so the
generated code's internal imports match its output location, one
subprocess.check_call into grpc_tools.protoc), adapted for plate_solver.proto
living outside this script's own directory (ps-grpc/proto/, not a proto/
nested under tools/parity/benchmark/).
"""

import subprocess
import sys
from pathlib import Path

# tools/parity/benchmark/ -- this script's own directory. Generated stubs are
# written under generated/ here so the harness can `from generated import
# plate_solver_pb2` without needing ps-grpc/proto/ on sys.path.
_benchmark_dir = Path(__file__).resolve().parent

# repo root is three levels up from tools/parity/benchmark/.
_repo_root = _benchmark_dir.parents[2]
_proto_root = _repo_root / "ps-grpc" / "proto"

# directory where generated code will be placed
_target_dir = _benchmark_dir / "generated"

# virtual include prefix so protoc's generated imports resolve relative to
# _benchmark_dir as "generated.plate_solver_pb2" would, mirroring
# cedar-solve's tetra3-prefix trick for a proto dir that isn't literally
# nested under the output root.
_include_prefix = _target_dir.relative_to(_benchmark_dir)


def main():
    _target_dir.mkdir(parents=True, exist_ok=True)
    proto_file = _proto_root / "plate_solver.proto"

    subprocess.check_call([
        sys.executable, "-m", "grpc_tools.protoc",
        f"-I{_include_prefix}={_proto_root}",
        f"--pyi_out={_benchmark_dir}",
        f"--python_out={_benchmark_dir}",
        f"--grpc_python_out={_benchmark_dir}",
        str(proto_file),
    ])


if __name__ == "__main__":
    main()
