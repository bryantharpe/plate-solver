"""
The eval-harness's shared image corpus.

11 images, sourced from reference-solutions/cedar-detect/test_data/ — chosen
because they're byte-identical (verified by md5sum) to the copies also
vendored under reference-solutions/cedar-solve/examples/data/medium_fov/, so
every system under comparison reads the exact same bytes with no extra
fixture to download or maintain. See
openspec/changes/feat-09-eval-harness/design.md for the verification.

9 astronomical images (real night-sky photos; a valid solve is expected) plus
2 non-astronomical "stress" images (no valid solve is expected — useful for
exercising each system's no-match path).
"""

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

# repo root is three levels up from tools/parity/benchmark/.
_repo_root = Path(__file__).resolve().parents[3]
TEST_DATA_DIR = _repo_root / "reference-solutions" / "cedar-detect" / "test_data"

Category = Literal["astronomical", "stress"]

# 8 real-sky photos plus hale_bopp.jpg. 2019-07-29T204726_Alt40_Azi-135_Try1.jpg
# is also tools/parity/capture_solve.py's existing reference image.
ASTRONOMICAL_IMAGES = [
    "2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi135_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi-45_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi45_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi-135_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi135_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi-45_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi45_Try1.jpg",
    "hale_bopp.jpg",
]

# Non-astronomical stress images: no valid solve is expected. Note
# reference-solutions/cedar-detect/test_data/ also has test_5mp_g100_e20ms.jpg
# and test_5mp_g100_e100ms.jpg, which are NOT part of the shared corpus (they
# aren't byte-identical to anything under cedar-solve's example data).
STRESS_IMAGES = [
    "tree.jpg",
    "test_5mp_g100_e50ms.jpg",
]


@dataclass(frozen=True)
class CorpusImage:
    name: str
    path: Path
    category: Category


CORPUS = [
    CorpusImage(name, TEST_DATA_DIR / name, "astronomical") for name in ASTRONOMICAL_IMAGES
] + [
    CorpusImage(name, TEST_DATA_DIR / name, "stress") for name in STRESS_IMAGES
]


def _self_check() -> None:
    assert len(ASTRONOMICAL_IMAGES) == 9, len(ASTRONOMICAL_IMAGES)
    assert len(STRESS_IMAGES) == 2, len(STRESS_IMAGES)
    assert len(CORPUS) == 11, len(CORPUS)
    missing = [str(img.path) for img in CORPUS if not img.path.is_file()]
    if missing:
        raise FileNotFoundError(f"corpus images not found: {missing}")
    print(f"corpus.py: {len(CORPUS)} images found under {TEST_DATA_DIR}")
    for img in CORPUS:
        print(f"  [{img.category:12s}] {img.name}")


if __name__ == "__main__":
    _self_check()
