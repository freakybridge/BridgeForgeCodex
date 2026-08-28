from __future__ import annotations

from typing import Any


class RegionMigrationError(ValueError):
    pass


def _marker_span(payload: bytes, begin: str, end: str) -> tuple[int, int]:
    lines = payload.splitlines(keepends=True)
    begin_bytes = begin.encode("utf-8")
    end_bytes = end.encode("utf-8")
    starts = [index for index, line in enumerate(lines) if line.rstrip(b"\r\n") == begin_bytes]
    stops = [index for index, line in enumerate(lines) if line.rstrip(b"\r\n") == end_bytes]
    if len(starts) != 1 or len(stops) != 1 or starts[0] >= stops[0]:
        raise RegionMigrationError(f"managed markers are missing, duplicated, or reversed: {begin} / {end}")
    start = sum(len(line) for line in lines[: starts[0]])
    stop = sum(len(line) for line in lines[: stops[0] + 1])
    return start, stop


def _marker_counts(payload: bytes, begin: str, end: str) -> tuple[int, int]:
    lines = payload.splitlines(keepends=True)
    begin_bytes = begin.encode("utf-8")
    end_bytes = end.encode("utf-8")
    return (
        sum(line.rstrip(b"\r\n") == begin_bytes for line in lines),
        sum(line.rstrip(b"\r\n") == end_bytes for line in lines),
    )


def _append_separator(payload: bytes) -> bytes:
    newline = b"\r\n" if b"\r\n" in payload and b"\n" not in payload.replace(b"\r\n", b"") else b"\n"
    if payload.endswith(newline + newline):
        return b""
    if payload.endswith(newline):
        return newline
    return newline + newline


def replace_or_append_region(
    source: bytes,
    current: bytes | None,
    region: dict[str, Any],
) -> bytes:
    """Candidate replacement for the synchronizer's region merge primitive."""

    missing_marker = str(region.get("missing_marker", "fail-closed"))
    if missing_marker not in {"fail-closed", "append"}:
        raise RegionMigrationError(f"unsupported missing-marker policy: {missing_marker}")

    begin = str(region["begin"])
    end = str(region["end"])
    source_start, source_stop = _marker_span(source, begin, end)
    source_block = source[source_start:source_stop]
    if current is None:
        return source
    counts = _marker_counts(current, begin, end)
    if counts == (0, 0):
        if missing_marker != "append":
            raise RegionMigrationError(f"managed markers are missing: {begin} / {end}")
        if current == b"":
            return source_block
        return current + _append_separator(current) + source_block
    if counts != (1, 1):
        raise RegionMigrationError(f"managed markers are missing, duplicated, or reversed: {begin} / {end}")

    current_start, current_stop = _marker_span(current, begin, end)
    return current[:current_start] + source_block + current[current_stop:]
