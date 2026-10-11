"""Turn inward-facing triangles of generated glbs outwards, in place.

TRELLIS meshes invert a share of their faces (41 % on abbey_byz_serbia). The DN
materials are single-sided, so Godot culls those faces: the model shows holes
through which its dark inside reads as a "dark back".

Each face votes over eight orthographic views (four yaws at 25° and 60°
elevation): seen from the front when it faces the camera, from behind otherwise.
Faces mostly seen from behind get their winding reversed; vertex normals of
those faces are negated when they disagree with the new winding. Only the index
and NORMAL buffers change; everything else in the glb is kept byte for byte.

Usage:
    uv run --project tools --with numpy --with pillow --with trimesh --with scipy
    python tools/experiments/fix_inverted_faces.py GLB [GLB ...] [--dry-run]
"""

import argparse
import json
import struct
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from retex import VIEW_SIZE, camera_basis, face_normals, project_view, rasterise  # noqa: E402

VOTE_YAWS = (0, 90, 180, 270)
VOTE_ELEVATIONS = (25.0, 60.0)
COMPONENT_DTYPES = {5121: np.uint8, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}


def read_glb(path: Path):
    """Return (json dict, binary chunk as bytearray)."""
    data = path.read_bytes()
    json_length = struct.unpack_from("<I", data, 12)[0]
    document = json.loads(data[20 : 20 + json_length])
    binary_start = 20 + json_length + 8
    binary_length = struct.unpack_from("<I", data, 20 + json_length)[0]
    return document, bytearray(data[binary_start : binary_start + binary_length])


def write_glb(path: Path, document: dict, binary: bytearray) -> None:
    """Rewrite a glb with the same json and an edited binary chunk."""
    data = path.read_bytes()
    json_length = struct.unpack_from("<I", data, 12)[0]
    binary_start = 20 + json_length + 8
    path.write_bytes(
        data[:binary_start] + bytes(binary) + data[binary_start + len(binary) :]
    )


def accessor_view(document: dict, binary: bytearray, index: int) -> np.ndarray:
    """Writable numpy view on a tightly packed accessor."""
    accessor = document["accessors"][index]
    buffer_view = document["bufferViews"][accessor["bufferView"]]
    dtype = COMPONENT_DTYPES[accessor["componentType"]]
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[accessor["type"]]
    stride = buffer_view.get("byteStride", 0)
    if stride and stride != np.dtype(dtype).itemsize * width:
        raise ValueError("interleaved buffer views are not supported")
    offset = buffer_view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    array = np.frombuffer(
        binary, dtype=dtype, count=accessor["count"] * width, offset=offset
    )
    return array.reshape(-1, width) if width > 1 else array


def inward_faces(positions: np.ndarray, faces: np.ndarray) -> np.ndarray:
    """Boolean mask of faces mostly seen from behind."""
    centre = (positions.min(0) + positions.max(0)) / 2
    unit = (positions - centre) / np.abs(positions - centre).max()
    normals = face_normals(unit, faces)
    score = np.zeros(len(faces))
    for elevation in VOTE_ELEVATIONS:
        for yaw in VOTE_YAWS:
            xy, depth = project_view(unit, faces, yaw, elevation)
            _, _, face_id = rasterise(
                xy, depth, np.zeros((*depth.shape, 0)), VIEW_SIZE, VIEW_SIZE
            )
            seen = np.zeros(len(faces), dtype=bool)
            seen[face_id[face_id >= 0]] = True
            _, _, forward = camera_basis(yaw, elevation)
            score += np.where(seen, -(normals @ forward), 0.0)
    return score < 0


def culled_holes(positions: np.ndarray, faces: np.ndarray) -> float:
    """Share of the silhouette where culling back faces (Godot) shows something behind the skin."""
    centre = (positions.min(0) + positions.max(0)) / 2
    unit = (positions - centre) / np.abs(positions - centre).max()
    normals = face_normals(unit, faces)
    covered, empty = 0, 0
    for yaw in VOTE_YAWS:
        _, _, forward = camera_basis(yaw, VOTE_ELEVATIONS[0])
        front = normals @ forward < 0
        xy, depth = project_view(unit, faces, yaw, VOTE_ELEVATIONS[0])
        no_attributes = np.zeros((*depth.shape, 0))
        full, _, _ = rasterise(xy, depth, no_attributes, VIEW_SIZE, VIEW_SIZE)
        culled, _, _ = rasterise(
            xy[front], depth[front], no_attributes[front], VIEW_SIZE, VIEW_SIZE
        )
        silhouette = np.isfinite(full)
        covered += int(silhouette.sum())
        empty += int((silhouette & ~(culled <= full + 0.02)).sum())
    return empty / max(covered, 1)


def fix_glb(path: Path, dry_run: bool) -> tuple[float, float, float]:
    """Fix one glb; return the share of reversed faces and the culled holes before/after."""
    document, binary = read_glb(path)
    total, reversed_count, holes_before, holes_after = 0, 0, [], []
    for mesh in document["meshes"]:
        for primitive in mesh["primitives"]:
            positions = accessor_view(
                document, binary, primitive["attributes"]["POSITION"]
            ).astype(np.float64)
            indices = accessor_view(document, binary, primitive["indices"])
            faces = indices.reshape(-1, 3).astype(np.int64)
            inward = inward_faces(positions, faces)
            holes_before.append(culled_holes(positions, faces))
            holes_after.append(
                culled_holes(
                    positions, np.where(inward[:, None], faces[:, ::-1], faces)
                )
            )
            if holes_after[-1] >= holes_before[-1]:  # no gain: leave it as it is
                inward[:] = False
                holes_after[-1] = holes_before[-1]
            total += len(faces)
            reversed_count += int(inward.sum())
            if dry_run or not inward.any():
                continue
            corrected = np.where(inward[:, None], faces[:, ::-1], faces)
            indices[:] = corrected.reshape(-1).astype(indices.dtype)
            if "NORMAL" in primitive["attributes"]:
                normals = accessor_view(
                    document, binary, primitive["attributes"]["NORMAL"]
                )
                face_n = face_normals(positions, corrected)
                summed = np.zeros_like(positions)
                np.add.at(summed, corrected.reshape(-1), np.repeat(face_n, 3, axis=0))
                disagree = np.einsum("ij,ij->i", normals, summed) < 0
                normals[disagree] *= -1
    if not dry_run and reversed_count:
        write_glb(path, document, binary)
    return (
        reversed_count / max(total, 1),
        float(np.mean(holes_before)),
        float(np.mean(holes_after)),
    )


def main() -> None:
    """Fix every glb given on the command line."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("glbs", nargs="+", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    for glb in args.glbs:
        reversed_share, before, after = fix_glb(glb, args.dry_run)
        print(
            f"{glb.name}\treversed {reversed_share:.3f}\tholes {before:.3f} -> {after:.3f}",
            flush=True,
        )


if __name__ == "__main__":
    main()
