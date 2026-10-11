"""TP: AI texture painting trial -- repaint a textured glb from depth-guided views.

``uv run --project tools --with trimesh --with pillow --with numpy --with scipy python
tools/experiments/retex.py --id city_west --glb game/assets/models/dn/buildings/city_west_lod0.glb
--prompt-file ~/dev/cent-ans-raw/dn/city_west/prompt.txt --out ~/dev/cent-ans-raw/tp/city_west``

Stages (each one is skipped when its outputs exist, delete them to redo):
1. render: orthographic views around the vertical axis (numpy z-buffer): depth map for the
   ControlNet, normals, current texture.
2. paint: Z-Image Turbo + ControlNet Union (local mflux), one image per view, same seed.
3. project: every texel gets its 3D position by rasterising the mesh in UV space; a texel seen by a
   view (z-buffer test) takes that view's colour with weight (n.v)^4; unseen texels keep the old
   texture. Painted colours are matched to the old texture's statistics (charter palette).
4. sheet: before/after, front and back, through ``tools/blender_scripts/ga3_compare_render.py``.
"""

from __future__ import annotations

import argparse
import fcntl
import json
import math
import re
import subprocess
from pathlib import Path

import numpy as np
import trimesh
from PIL import Image
from scipy import ndimage

REPO = Path(__file__).resolve().parents[2]
VIEW_SIZE = 768
GPU_LOCK = Path.home() / "dev/cent-ans-raw/dn/gpu.lock"
MFLUX_PYTHON = Path.home() / ".local/share/uv/tools/mflux/bin/python"
TEXTURE_WEIGHT_POWER = 4.0
DARK_ONLY_THRESHOLD = (
    0.8  # share of the good side's mean luminance under which a texel is repainted
)
DARK_ONLY_BLUR = 3.0  # texels; softens the repaint mask edges
FRAME_EXTENT = 1.4  # a [-1, 1]^3 box seen 25 deg from above spans +-1.33 vertically
VIEW_NAMES = {0: "front", 90: "right side", 180: "back", 270: "left side"}


def load_mesh(glb: Path):
    """Return (scene, geometry, unit vertices, faces, uv, texture array)."""
    scene = trimesh.load(glb)
    geometry = next(iter(scene.geometry.values()))
    vertices = np.asarray(geometry.vertices, dtype=np.float64)
    centre = (vertices.min(0) + vertices.max(0)) / 2
    unit = (vertices - centre) / np.abs(vertices - centre).max()  # fits [-1, 1]^3
    texture = (
        np.asarray(
            geometry.visual.material.baseColorTexture.convert("RGB"), dtype=np.float32
        )
        / 255
    )
    return (
        scene,
        geometry,
        unit,
        np.asarray(geometry.faces),
        np.asarray(geometry.visual.uv),
        texture,
    )


def camera_basis(yaw_deg: float, elevation_deg: float):
    """Right, up and forward vectors of an orthographic camera turning around +Y."""
    yaw, elevation = math.radians(yaw_deg), math.radians(elevation_deg)
    to_camera = np.array(
        [
            math.sin(yaw) * math.cos(elevation),
            math.sin(elevation),
            -math.cos(yaw) * math.cos(elevation),
        ]
    )
    forward = -to_camera
    right = np.cross(forward, [0.0, 1.0, 0.0])
    right /= np.linalg.norm(right)
    up = np.cross(right, forward)
    return right, up, forward


def rasterise(
    xy: np.ndarray, depth: np.ndarray, attributes: np.ndarray, width: int, height: int
):
    """Z-buffer triangles. xy (N,3,2) pixels, depth (N,3), attributes (N,3,C) -> (zbuf, image, face id)."""
    zbuffer = np.full((height, width), np.inf)
    image = np.zeros((height, width, attributes.shape[2]))
    face_id = np.full((height, width), -1, dtype=np.int64)
    for index in range(len(xy)):
        (x0, y0), (x1, y1), (x2, y2) = xy[index]
        area = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0)
        if abs(area) < 1e-12:
            continue
        left, right = (
            max(int(math.floor(min(x0, x1, x2))), 0),
            min(int(math.ceil(max(x0, x1, x2))), width - 1),
        )
        top, bottom = (
            max(int(math.floor(min(y0, y1, y2))), 0),
            min(int(math.ceil(max(y0, y1, y2))), height - 1),
        )
        if left > right or top > bottom:
            continue
        grid_y, grid_x = np.mgrid[top : bottom + 1, left : right + 1] + 0.5
        weight0 = ((x1 - grid_x) * (y2 - grid_y) - (x2 - grid_x) * (y1 - grid_y)) / area
        weight1 = ((x2 - grid_x) * (y0 - grid_y) - (x0 - grid_x) * (y2 - grid_y)) / area
        weight2 = 1.0 - weight0 - weight1
        inside = (weight0 >= -1e-6) & (weight1 >= -1e-6) & (weight2 >= -1e-6)
        if not inside.any():
            continue
        z = (
            weight0 * depth[index, 0]
            + weight1 * depth[index, 1]
            + weight2 * depth[index, 2]
        )
        window = zbuffer[top : bottom + 1, left : right + 1]
        closer = inside & (z < window)
        if not closer.any():
            continue
        window[closer] = z[closer]
        values = (
            weight0[..., None] * attributes[index, 0]
            + weight1[..., None] * attributes[index, 1]
            + weight2[..., None] * attributes[index, 2]
        )
        image[top : bottom + 1, left : right + 1][closer] = values[closer]
        face_id[top : bottom + 1, left : right + 1][closer] = index
    return zbuffer, image, face_id


def project_view(unit, faces, yaw, elevation):
    """Pixel xy (N,3,2) and depth (N,3) of every triangle corner for one view."""
    right, up, forward = camera_basis(yaw, elevation)
    corners = unit[faces]
    half = VIEW_SIZE / 2 / FRAME_EXTENT
    x = corners @ right * half + VIEW_SIZE / 2
    y = VIEW_SIZE / 2 - corners @ up * half
    return np.stack([x, y], -1), corners @ forward


def face_normals(unit, faces):
    """Unit normal per face."""
    corners = unit[faces]
    normals = np.cross(corners[:, 1] - corners[:, 0], corners[:, 2] - corners[:, 0])
    return normals / np.maximum(np.linalg.norm(normals, axis=1, keepdims=True), 1e-12)


def sample(texture, uv):
    """Nearest texel of a texture (trimesh uv, origin bottom-left)."""
    height, width = texture.shape[:2]
    column = np.clip((uv[..., 0] % 1.0) * width, 0, width - 1).astype(int)
    row = np.clip((1.0 - uv[..., 1] % 1.0) * height, 0, height - 1).astype(int)
    return texture[row, column]


def stage_render(mesh, out: Path, yaws, elevation):
    """Depth, normal and current-texture images for every view (+ cached z-buffers)."""
    _, _, unit, faces, uv, texture = mesh
    normals = face_normals(unit, faces)
    for yaw in yaws:
        target = out / f"view_{yaw:03d}_depth.png"
        if target.exists():
            continue
        xy, depth = project_view(unit, faces, yaw, elevation)
        attributes = np.concatenate(
            [uv[faces], np.repeat(normals[:, None, :], 3, axis=1)], axis=2
        )
        zbuffer, image, face_id = rasterise(xy, depth, attributes, VIEW_SIZE, VIEW_SIZE)
        mask = face_id >= 0
        near, far = zbuffer[mask].min(), zbuffer[mask].max()
        depth_image = np.zeros_like(zbuffer)
        depth_image[mask] = 1.0 - 0.8 * (zbuffer[mask] - near) / max(
            far - near, 1e-9
        )  # near = white
        colour = np.full((VIEW_SIZE, VIEW_SIZE, 3), 0.5)
        colour[mask] = sample(texture, image[mask][:, :2])
        normal_image = np.zeros((VIEW_SIZE, VIEW_SIZE, 3))
        normal_image[mask] = image[mask][:, 2:5] * 0.5 + 0.5
        np.save(
            out / f"view_{yaw:03d}_zbuffer.npy",
            np.where(mask, zbuffer, np.inf).astype(np.float32),
        )
        Image.fromarray((colour * 255).astype(np.uint8)).save(
            out / f"view_{yaw:03d}_old.png"
        )
        Image.fromarray((normal_image * 255).astype(np.uint8)).save(
            out / f"view_{yaw:03d}_normal.png"
        )
        Image.fromarray((depth_image * 255).astype(np.uint8)).convert("RGB").save(
            target
        )
        print(f"render yaw {yaw}: {mask.mean():.0%} of the frame", flush=True)


def view_prompt(base_prompt: str, yaw: int) -> str:
    """Asset prompt with the framing sentence replaced by this view's orthographic framing."""
    body = re.split(
        r"\.\s+(?=Three-quarter view|Front view|Side view)", base_prompt.strip()
    )[0].rstrip(".")
    return (
        f"{body}. Orthographic {VIEW_NAMES.get(yaw, 'side')} view from slightly above, the whole object "
        "centred, on a plain uniform medium grey studio background, soft even diffuse overcast light, "
        "no baked directional shadow, no cast shadow, no reflections, sharp realistic material detail."
    )


def stage_paint(
    out: Path,
    yaws,
    base_prompt: str,
    seed: int,
    steps: int,
    strength: float,
    model: str,
):
    """One depth-guided Z-Image Turbo image per view."""
    for yaw in yaws:
        target = out / f"view_{yaw:03d}_paint.png"
        if target.exists():
            continue
        prompt = view_prompt(base_prompt, yaw)
        (out / f"view_{yaw:03d}_prompt.txt").write_text(prompt)
        command = [
            str(MFLUX_PYTHON), str(Path(__file__).with_name("retex_mflux.py")), "--model", model,
            "--base-model", "z-image-turbo-controlnet-union-2.1", "--quantize", "8",
            "--prompt", prompt, "--seed", str(seed), "--steps", str(steps),
            "--width", str(VIEW_SIZE), "--height", str(VIEW_SIZE),
            "--control", f"depth:{out / f'view_{yaw:03d}_depth.png'}:{strength}",
            "--output", str(target),
        ]  # fmt: skip
        print("paint", yaw, flush=True)
        with GPU_LOCK.open(
            "w"
        ) as lock:  # shared with dn_batch: one local GPU job at a time
            fcntl.flock(lock, fcntl.LOCK_EX)
            subprocess.run(command, check=True)


def match_colours(source: np.ndarray, reference: np.ndarray) -> np.ndarray:
    """Per-channel mean/std transfer (pixel lists, 0-1)."""
    matched = (source - source.mean(0)) / np.maximum(
        source.std(0), 1e-6
    ) * reference.std(0) + reference.mean(0)
    return np.clip(matched, 0, 1)


def brightest_reference(out: Path, yaws) -> np.ndarray:
    """Original pixels of the brightest view: the well-textured side holds the palette.

    The broken side is the dark one, so matching to an arbitrary first view can copy its gloom.
    """
    candidates = []
    for yaw in yaws:
        mask = np.isfinite(np.load(out / f"view_{yaw:03d}_zbuffer.npy"))
        old_view = (
            np.asarray(
                Image.open(out / f"view_{yaw:03d}_old.png").convert("RGB"),
                dtype=np.float32,
            )
            / 255
        )
        candidates.append(old_view[mask])
    return max(candidates, key=lambda pixels: pixels.mean())


def stage_project(
    mesh, out: Path, yaws, elevation, texture_size: int, dark_only: bool = False
):
    """Bake painted views into a new base-colour texture."""
    target = out / "texture_new.png"
    if target.exists():
        return
    _, _, unit, faces, uv, old_texture = mesh
    normals = face_normals(unit, faces)
    texel_xy = np.stack(
        [uv[faces][..., 0] * texture_size, (1.0 - uv[faces][..., 1]) * texture_size], -1
    )
    attributes = np.concatenate(
        [unit[faces], np.repeat(normals[:, None, :], 3, axis=1)], axis=2
    )
    _, texels, texel_face = rasterise(
        texel_xy, np.zeros(texel_xy.shape[:2]), attributes, texture_size, texture_size
    )
    covered = texel_face >= 0
    positions, texel_normals = texels[covered][:, :3], texels[covered][:, 3:6]
    old = (
        np.asarray(
            Image.fromarray((old_texture * 255).astype(np.uint8)).resize(
                (texture_size, texture_size)
            ),
            dtype=np.float32,
        )
        / 255
    )
    accumulated = np.zeros((len(positions), 3))
    total_weight = np.zeros(len(positions))
    reference = brightest_reference(out, yaws)
    for yaw in yaws:
        right, up, forward = camera_basis(yaw, elevation)
        zbuffer = np.load(out / f"view_{yaw:03d}_zbuffer.npy")
        painted = (
            np.asarray(
                Image.open(out / f"view_{yaw:03d}_paint.png")
                .convert("RGB")
                .resize((VIEW_SIZE, VIEW_SIZE)),
                dtype=np.float32,
            )
            / 255
        )
        mask = np.isfinite(zbuffer)
        painted[mask] = match_colours(painted[mask], reference)
        Image.fromarray((painted * 255).astype(np.uint8)).save(
            out / f"view_{yaw:03d}_paint_matched.png"
        )
        half = VIEW_SIZE / 2 / FRAME_EXTENT
        column = np.clip(
            (positions @ right * half + VIEW_SIZE / 2).astype(int), 0, VIEW_SIZE - 1
        )
        row = np.clip(
            (VIEW_SIZE / 2 - positions @ up * half).astype(int), 0, VIEW_SIZE - 1
        )
        depth = positions @ forward
        visible = depth <= zbuffer[row, column] + 0.01
        # abs: TRELLIS meshes mix face windings; the z-buffer already rejects hidden texels.
        facing = np.abs(texel_normals @ forward)
        weight = np.where(visible, facing**TEXTURE_WEIGHT_POWER, 0.0)
        accumulated += weight[:, None] * painted[row, column]
        total_weight += weight
    new = old.copy()
    seen = total_weight > 1e-3
    covered_rows, covered_columns = np.nonzero(covered)
    blended = accumulated[seen] / total_weight[seen, None]
    if dark_only:
        # Keep texels already as bright as the good side; repaint the dark ones fully.
        luminance = old.mean(2)
        threshold = DARK_ONLY_THRESHOLD * reference.mean()
        repaint = np.clip((threshold - luminance) / (0.5 * threshold), 0, 1)
        repaint = ndimage.gaussian_filter(repaint, DARK_ONLY_BLUR)
        share = repaint[covered_rows[seen], covered_columns[seen], None]
        old_texels = old[covered_rows[seen], covered_columns[seen]]
        blended = share * blended + (1 - share) * old_texels
    new[covered_rows[seen], covered_columns[seen]] = blended
    # Seam padding: texels outside every UV island copy their nearest island texel.
    _, (near_rows, near_columns) = ndimage.distance_transform_edt(
        ~covered, return_indices=True
    )
    new = new[near_rows, near_columns]
    Image.fromarray((new * 255).astype(np.uint8)).save(target)
    report = {
        "texels_covered": int(covered.sum()),
        "texels_repainted": int(seen.sum()),
        "share_repainted": round(float(seen.mean()), 3),
    }
    (out / "project.json").write_text(json.dumps(report, indent=1))
    print("project", report, flush=True)


def inverted_faces(mesh, out: Path, yaws, elevation) -> np.ndarray:
    """Faces that the views see from behind: their winding points inwards.

    TRELLIS meshes invert a large share of faces (41 % on abbey_byz_serbia); lit from
    the wrong side they read as a dark, dirty texture.
    """
    _, _, unit, faces, _, _ = mesh
    normals = face_normals(unit, faces)
    centres = unit[faces].mean(1)
    half = VIEW_SIZE / 2 / FRAME_EXTENT
    facing_score = np.zeros(len(faces))
    for yaw in yaws:
        right, up, forward = camera_basis(yaw, elevation)
        zbuffer = np.load(out / f"view_{yaw:03d}_zbuffer.npy")
        column = np.clip(
            (centres @ right * half + VIEW_SIZE / 2).astype(int), 0, VIEW_SIZE - 1
        )
        row = np.clip(
            (VIEW_SIZE / 2 - centres @ up * half).astype(int), 0, VIEW_SIZE - 1
        )
        visible = centres @ forward <= zbuffer[row, column] + 0.01
        facing_score += np.where(visible, -(normals @ forward), 0.0)
    return facing_score < 0


def stage_export(mesh, out: Path, yaws, elevation) -> tuple[Path, Path]:
    """Glbs with inverted faces turned outwards: original texture, then repainted."""
    scene, geometry, *_ = mesh
    flip = inverted_faces(mesh, out, yaws, elevation)
    geometry.faces = np.where(flip[:, None], geometry.faces[:, ::-1], geometry.faces)
    print("export", {"inverted_faces": round(float(flip.mean()), 3)}, flush=True)
    normals_only = out / "normals_only.glb"
    scene.export(normals_only)
    target = out / "retex.glb"
    geometry.visual.material.baseColorTexture = Image.open(
        out / "texture_new.png"
    ).convert("RGB")
    scene.export(target)
    return normals_only, target


def stage_sheet(
    model_id: str, old_glb: Path, normals_glb: Path, new_glb: Path, out: Path
):
    """Before, normals fixed only, and repainted (front and back) in one Blender sheet."""
    command = [
        "blender", "-b", "--factory-startup", "-P", str(REPO / "tools/blender_scripts/ga3_compare_render.py"), "--",
        "--out", str(out / "sheet.png"), "--fit-width", "1.8",
        "--panel", str(old_glb), f"{model_id} avant face", "--yaw", "0",
        "--panel", str(old_glb), "avant dos", "--yaw", "180",
        "--panel", str(normals_glb), "normales face", "--yaw", "0",
        "--panel", str(normals_glb), "normales dos", "--yaw", "180",
        "--panel", str(new_glb), "après face", "--yaw", "0",
        "--panel", str(new_glb), "après dos", "--yaw", "180",
    ]  # fmt: skip
    subprocess.run(command, capture_output=True, check=False)


def contact(out: Path, yaws):
    """Views strip: depth / old / painted (matched) per view."""
    rows = []
    for yaw in yaws:
        tiles = [
            Image.open(out / f"view_{yaw:03d}_{kind}.png")
            .convert("RGB")
            .resize((256, 256))
            for kind in ("depth", "old", "paint_matched")
        ]
        row = Image.new("RGB", (768, 256))
        for index, tile in enumerate(tiles):
            row.paste(tile, (index * 256, 0))
        rows.append(row)
    strip = Image.new("RGB", (768, 256 * len(rows)))
    for index, row in enumerate(rows):
        strip.paste(row, (0, index * 256))
    strip.save(out / "views.png")


def main() -> None:
    """CLI."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--id", required=True)
    parser.add_argument("--glb", required=True, type=Path)
    parser.add_argument("--prompt-file", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--yaws", default="0,90,180,270")
    parser.add_argument("--elevation", type=float, default=25.0)
    parser.add_argument("--seed", type=int, default=1337)
    parser.add_argument("--steps", type=int, default=9)
    parser.add_argument("--strength", type=float, default=0.85)
    parser.add_argument(
        "--model",
        default=str(Path.home() / "models/mflux/z-image-turbo-q8"),
        help="local Z-Image Turbo base (the ControlNet itself comes from HF, 6.7 GB once)",
    )
    parser.add_argument("--texture-size", type=int, default=1024)
    parser.add_argument("--render-only", action="store_true")
    parser.add_argument(
        "--dark-only",
        action="store_true",
        help="repaint only texels darker than the brightest original view",
    )
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    yaws = [int(value) for value in args.yaws.split(",")]
    mesh = load_mesh(args.glb)
    stage_render(mesh, args.out, yaws, args.elevation)
    if args.render_only:
        return
    stage_paint(
        args.out,
        yaws,
        args.prompt_file.read_text(),
        args.seed,
        args.steps,
        args.strength,
        args.model,
    )
    stage_project(
        mesh, args.out, yaws, args.elevation, args.texture_size, args.dark_only
    )
    normals_glb, new_glb = stage_export(mesh, args.out, yaws, args.elevation)
    contact(args.out, yaws)
    stage_sheet(args.id, args.glb, normals_glb, new_glb, args.out)
    print("done", args.out, flush=True)


if __name__ == "__main__":
    main()
