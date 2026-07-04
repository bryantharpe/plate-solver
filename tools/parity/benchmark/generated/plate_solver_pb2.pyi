import datetime

from google.protobuf import duration_pb2 as _duration_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class SolveStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MATCH_FOUND: _ClassVar[SolveStatus]
    NO_MATCH: _ClassVar[SolveStatus]
    TIMEOUT: _ClassVar[SolveStatus]
    CANCELLED: _ClassVar[SolveStatus]
    TOO_FEW: _ClassVar[SolveStatus]
MATCH_FOUND: SolveStatus
NO_MATCH: SolveStatus
TIMEOUT: SolveStatus
CANCELLED: SolveStatus
TOO_FEW: SolveStatus

class Image(_message.Message):
    __slots__ = ("width", "height", "image_data", "shmem_name", "reopen_shmem")
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    IMAGE_DATA_FIELD_NUMBER: _ClassVar[int]
    SHMEM_NAME_FIELD_NUMBER: _ClassVar[int]
    REOPEN_SHMEM_FIELD_NUMBER: _ClassVar[int]
    width: int
    height: int
    image_data: bytes
    shmem_name: str
    reopen_shmem: bool
    def __init__(self, width: _Optional[int] = ..., height: _Optional[int] = ..., image_data: _Optional[bytes] = ..., shmem_name: _Optional[str] = ..., reopen_shmem: _Optional[bool] = ...) -> None: ...

class ImageCoord(_message.Message):
    __slots__ = ("x", "y")
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    x: float
    y: float
    def __init__(self, x: _Optional[float] = ..., y: _Optional[float] = ...) -> None: ...

class StarCentroid(_message.Message):
    __slots__ = ("centroid_position", "brightness", "num_saturated")
    CENTROID_POSITION_FIELD_NUMBER: _ClassVar[int]
    BRIGHTNESS_FIELD_NUMBER: _ClassVar[int]
    NUM_SATURATED_FIELD_NUMBER: _ClassVar[int]
    centroid_position: ImageCoord
    brightness: float
    num_saturated: int
    def __init__(self, centroid_position: _Optional[_Union[ImageCoord, _Mapping]] = ..., brightness: _Optional[float] = ..., num_saturated: _Optional[int] = ...) -> None: ...

class Rectangle(_message.Message):
    __slots__ = ("origin_x", "origin_y", "width", "height")
    ORIGIN_X_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_Y_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    origin_x: int
    origin_y: int
    width: int
    height: int
    def __init__(self, origin_x: _Optional[int] = ..., origin_y: _Optional[int] = ..., width: _Optional[int] = ..., height: _Optional[int] = ...) -> None: ...

class CentroidsRequest(_message.Message):
    __slots__ = ("input_image", "sigma", "binning", "return_binned", "use_binned_for_star_candidates", "detect_hot_pixels", "normalize_rows", "estimate_background_region")
    INPUT_IMAGE_FIELD_NUMBER: _ClassVar[int]
    SIGMA_FIELD_NUMBER: _ClassVar[int]
    BINNING_FIELD_NUMBER: _ClassVar[int]
    RETURN_BINNED_FIELD_NUMBER: _ClassVar[int]
    USE_BINNED_FOR_STAR_CANDIDATES_FIELD_NUMBER: _ClassVar[int]
    DETECT_HOT_PIXELS_FIELD_NUMBER: _ClassVar[int]
    NORMALIZE_ROWS_FIELD_NUMBER: _ClassVar[int]
    ESTIMATE_BACKGROUND_REGION_FIELD_NUMBER: _ClassVar[int]
    input_image: Image
    sigma: float
    binning: int
    return_binned: bool
    use_binned_for_star_candidates: bool
    detect_hot_pixels: bool
    normalize_rows: bool
    estimate_background_region: Rectangle
    def __init__(self, input_image: _Optional[_Union[Image, _Mapping]] = ..., sigma: _Optional[float] = ..., binning: _Optional[int] = ..., return_binned: _Optional[bool] = ..., use_binned_for_star_candidates: _Optional[bool] = ..., detect_hot_pixels: _Optional[bool] = ..., normalize_rows: _Optional[bool] = ..., estimate_background_region: _Optional[_Union[Rectangle, _Mapping]] = ...) -> None: ...

class CentroidsResult(_message.Message):
    __slots__ = ("noise_estimate", "background_estimate", "hot_pixel_count", "peak_star_pixel", "star_candidates", "binned_image", "algorithm_time")
    NOISE_ESTIMATE_FIELD_NUMBER: _ClassVar[int]
    BACKGROUND_ESTIMATE_FIELD_NUMBER: _ClassVar[int]
    HOT_PIXEL_COUNT_FIELD_NUMBER: _ClassVar[int]
    PEAK_STAR_PIXEL_FIELD_NUMBER: _ClassVar[int]
    STAR_CANDIDATES_FIELD_NUMBER: _ClassVar[int]
    BINNED_IMAGE_FIELD_NUMBER: _ClassVar[int]
    ALGORITHM_TIME_FIELD_NUMBER: _ClassVar[int]
    noise_estimate: float
    background_estimate: float
    hot_pixel_count: int
    peak_star_pixel: int
    star_candidates: _containers.RepeatedCompositeFieldContainer[StarCentroid]
    binned_image: Image
    algorithm_time: _duration_pb2.Duration
    def __init__(self, noise_estimate: _Optional[float] = ..., background_estimate: _Optional[float] = ..., hot_pixel_count: _Optional[int] = ..., peak_star_pixel: _Optional[int] = ..., star_candidates: _Optional[_Iterable[_Union[StarCentroid, _Mapping]]] = ..., binned_image: _Optional[_Union[Image, _Mapping]] = ..., algorithm_time: _Optional[_Union[datetime.timedelta, _duration_pb2.Duration, _Mapping]] = ...) -> None: ...

class SolveParams(_message.Message):
    __slots__ = ("fov_estimate", "fov_max_error", "match_radius", "match_threshold", "solve_timeout_ms", "distortion", "return_matches", "return_catalog")
    FOV_ESTIMATE_FIELD_NUMBER: _ClassVar[int]
    FOV_MAX_ERROR_FIELD_NUMBER: _ClassVar[int]
    MATCH_RADIUS_FIELD_NUMBER: _ClassVar[int]
    MATCH_THRESHOLD_FIELD_NUMBER: _ClassVar[int]
    SOLVE_TIMEOUT_MS_FIELD_NUMBER: _ClassVar[int]
    DISTORTION_FIELD_NUMBER: _ClassVar[int]
    RETURN_MATCHES_FIELD_NUMBER: _ClassVar[int]
    RETURN_CATALOG_FIELD_NUMBER: _ClassVar[int]
    fov_estimate: float
    fov_max_error: float
    match_radius: float
    match_threshold: float
    solve_timeout_ms: int
    distortion: float
    return_matches: bool
    return_catalog: bool
    def __init__(self, fov_estimate: _Optional[float] = ..., fov_max_error: _Optional[float] = ..., match_radius: _Optional[float] = ..., match_threshold: _Optional[float] = ..., solve_timeout_ms: _Optional[int] = ..., distortion: _Optional[float] = ..., return_matches: _Optional[bool] = ..., return_catalog: _Optional[bool] = ...) -> None: ...

class SolveFromCentroidsRequest(_message.Message):
    __slots__ = ("centroids", "width", "height", "params")
    CENTROIDS_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    PARAMS_FIELD_NUMBER: _ClassVar[int]
    centroids: _containers.RepeatedCompositeFieldContainer[ImageCoord]
    width: int
    height: int
    params: SolveParams
    def __init__(self, centroids: _Optional[_Iterable[_Union[ImageCoord, _Mapping]]] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., params: _Optional[_Union[SolveParams, _Mapping]] = ...) -> None: ...

class SolveFromImageRequest(_message.Message):
    __slots__ = ("extract", "params")
    EXTRACT_FIELD_NUMBER: _ClassVar[int]
    PARAMS_FIELD_NUMBER: _ClassVar[int]
    extract: CentroidsRequest
    params: SolveParams
    def __init__(self, extract: _Optional[_Union[CentroidsRequest, _Mapping]] = ..., params: _Optional[_Union[SolveParams, _Mapping]] = ...) -> None: ...

class MatchedStar(_message.Message):
    __slots__ = ("centroid", "ra", "dec", "mag", "cat_id")
    CENTROID_FIELD_NUMBER: _ClassVar[int]
    RA_FIELD_NUMBER: _ClassVar[int]
    DEC_FIELD_NUMBER: _ClassVar[int]
    MAG_FIELD_NUMBER: _ClassVar[int]
    CAT_ID_FIELD_NUMBER: _ClassVar[int]
    centroid: ImageCoord
    ra: float
    dec: float
    mag: float
    cat_id: int
    def __init__(self, centroid: _Optional[_Union[ImageCoord, _Mapping]] = ..., ra: _Optional[float] = ..., dec: _Optional[float] = ..., mag: _Optional[float] = ..., cat_id: _Optional[int] = ...) -> None: ...

class Solution(_message.Message):
    __slots__ = ("status", "ra", "dec", "roll", "fov", "distortion", "rmse", "p90e", "maxe", "matches", "prob", "t_extract_ms", "t_solve_ms", "matched")
    STATUS_FIELD_NUMBER: _ClassVar[int]
    RA_FIELD_NUMBER: _ClassVar[int]
    DEC_FIELD_NUMBER: _ClassVar[int]
    ROLL_FIELD_NUMBER: _ClassVar[int]
    FOV_FIELD_NUMBER: _ClassVar[int]
    DISTORTION_FIELD_NUMBER: _ClassVar[int]
    RMSE_FIELD_NUMBER: _ClassVar[int]
    P90E_FIELD_NUMBER: _ClassVar[int]
    MAXE_FIELD_NUMBER: _ClassVar[int]
    MATCHES_FIELD_NUMBER: _ClassVar[int]
    PROB_FIELD_NUMBER: _ClassVar[int]
    T_EXTRACT_MS_FIELD_NUMBER: _ClassVar[int]
    T_SOLVE_MS_FIELD_NUMBER: _ClassVar[int]
    MATCHED_FIELD_NUMBER: _ClassVar[int]
    status: SolveStatus
    ra: float
    dec: float
    roll: float
    fov: float
    distortion: float
    rmse: float
    p90e: float
    maxe: float
    matches: int
    prob: float
    t_extract_ms: float
    t_solve_ms: float
    matched: _containers.RepeatedCompositeFieldContainer[MatchedStar]
    def __init__(self, status: _Optional[_Union[SolveStatus, str]] = ..., ra: _Optional[float] = ..., dec: _Optional[float] = ..., roll: _Optional[float] = ..., fov: _Optional[float] = ..., distortion: _Optional[float] = ..., rmse: _Optional[float] = ..., p90e: _Optional[float] = ..., maxe: _Optional[float] = ..., matches: _Optional[int] = ..., prob: _Optional[float] = ..., t_extract_ms: _Optional[float] = ..., t_solve_ms: _Optional[float] = ..., matched: _Optional[_Iterable[_Union[MatchedStar, _Mapping]]] = ...) -> None: ...

class InfoRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ServerInfo(_message.Message):
    __slots__ = ("version", "star_catalog", "min_fov", "max_fov", "num_patterns", "epoch_equinox", "epoch_proper_motion")
    VERSION_FIELD_NUMBER: _ClassVar[int]
    STAR_CATALOG_FIELD_NUMBER: _ClassVar[int]
    MIN_FOV_FIELD_NUMBER: _ClassVar[int]
    MAX_FOV_FIELD_NUMBER: _ClassVar[int]
    NUM_PATTERNS_FIELD_NUMBER: _ClassVar[int]
    EPOCH_EQUINOX_FIELD_NUMBER: _ClassVar[int]
    EPOCH_PROPER_MOTION_FIELD_NUMBER: _ClassVar[int]
    version: str
    star_catalog: str
    min_fov: float
    max_fov: float
    num_patterns: int
    epoch_equinox: float
    epoch_proper_motion: float
    def __init__(self, version: _Optional[str] = ..., star_catalog: _Optional[str] = ..., min_fov: _Optional[float] = ..., max_fov: _Optional[float] = ..., num_patterns: _Optional[int] = ..., epoch_equinox: _Optional[float] = ..., epoch_proper_motion: _Optional[float] = ...) -> None: ...
