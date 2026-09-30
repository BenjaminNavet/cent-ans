"""HB5: LODs of spiky TRELLIS rocks through a coarse voxel remesh (Blender, headless).

``ga3_cleanup.py`` collapse-decimates the source; on needle and serrated shapes (alpine spires,
stratified ridges) Blender's collapse stalls far above the target (thin tubes cannot collapse
without degenerate topology), and its voxel fallback (size / 120) keeps the needles. Here each
LOD is a voxel remesh of the cleaned source at ``max(dimensions) / div`` (``--divs``, one per
LOD: coarser for the far LODs), then collapsed to its target, seated and baked from the
untouched source with the same helpers and arguments as ``ga3_cleanup.py``::

    blender -b --factory-startup --python tools/blender_scripts/hb_rock_lods.py -- \
        SOURCE.glb OUT_DIR NAME [ga3_cleanup.py options] --divs 64,40,22

Called by ``hb_rock_outcrops.py cleanup`` for outcrops with ``voxel_divs`` in the catalogue.
"""

import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ga3_cleanup as ga3  # noqa: E402


def split_divs() -> list[int]:
    """Remove ``--divs A,B,C`` from ``sys.argv`` (``ga3.parse_args`` rejects it)."""
    argv = sys.argv
    if "--divs" not in argv:
        return [64, 40, 22]
    at = argv.index("--divs")
    divs = [int(v) for v in argv[at + 1].split(",")]
    del argv[at : at + 2]
    return divs


def voxel_low(
    source: bpy.types.Object, name: str, target: int, div: int
) -> bpy.types.Object:
    """Copy of ``source``, voxel-remeshed at ``max(dims) / div`` then collapsed to ``target``."""
    obj = source.copy()
    obj.data = source.data.copy()
    obj.name = name
    bpy.context.scene.collection.objects.link(obj)
    for layer in list(obj.data.uv_layers):
        obj.data.uv_layers.remove(layer)
    ga3.select_only(obj)
    remesh = obj.modifiers.new("vox", "REMESH")
    remesh.mode = "VOXEL"
    remesh.voxel_size = max(obj.dimensions) / div
    bpy.ops.object.modifier_apply(modifier=remesh.name)
    ga3.collapse_to(obj, target)
    return obj


def main() -> None:
    """Run the voxel LOD pipeline."""
    divs = split_divs()
    args = ga3.parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    source = ga3.import_joined(args.source)
    for slot in source.material_slots:
        bsdf = (
            slot.material.node_tree.nodes.get("Principled BSDF")
            if slot.material
            else None
        )
        if bsdf is not None:
            for link in list(bsdf.inputs["Metallic"].links):
                slot.material.node_tree.links.remove(link)
            bsdf.inputs["Metallic"].default_value = 0.0
    ga3.clean(source, args.island_min)
    ga3.align(source, args.align, args.yaw)
    ga3.normalise(source, args.length, args.scale_axis)
    if args.strip_base > 0.0:
        ga3.strip_base(source, args.strip_base, args.length)
        ga3.normalise(source, args.length, args.scale_axis)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    for level, fraction in enumerate(ga3.LOD_FRACTIONS):
        target = int(args.lod0 * fraction)
        name = f"{args.name}_lod{level}"
        low = voxel_low(source, name, target, divs[level])
        ga3.seat(low)
        ga3.bake_maps(source, low, max(128, args.tex >> level), args)
        ga3.export(low, out_dir / f"{name}.glb")
        dims = low.dimensions
        print(
            f"GA3 {name}: {ga3.triangle_count(low)} tris (voxel /{divs[level]}), "
            f"{dims.x:.2f} x {dims.y:.2f} x {dims.z:.2f} m"
        )


if __name__ == "__main__":
    main()
