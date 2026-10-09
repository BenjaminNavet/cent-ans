"""QW-D : toute texture de modèle 3D (et l'eau) est importée en VRAM compressé + mipmaps."""

from pathlib import Path

from cent_ans_tools import texture_import_rules as rules

SAMPLE = """[remap]

importer="texture"
type="CompressedTexture2D"

[params]

compress/mode=0
compress/normal_map=0
mipmaps/generate=false
detect_3d/compress_to=1
"""


def test_fix_import_rewrites_params(tmp_path: Path) -> None:
    normal = tmp_path / "x_normal.png.import"
    normal.write_text(SAMPLE, encoding="utf-8")
    assert rules.violations(normal)
    assert rules.fix_import(normal)
    text = normal.read_text(encoding="utf-8")
    assert "compress/mode=2" in text
    assert "compress/normal_map=1" in text
    assert "mipmaps/generate=true" in text
    assert not rules.violations(normal)
    assert not rules.fix_import(normal)


def test_albedo_keeps_normal_map_detection(tmp_path: Path) -> None:
    albedo = tmp_path / "m_Image_0.jpg.import"
    albedo.write_text(SAMPLE, encoding="utf-8")
    rules.fix_import(albedo)
    assert "compress/normal_map=0" in albedo.read_text(encoding="utf-8")


def test_repo_model_textures_are_vram_compressed() -> None:
    bad = {
        str(path.relative_to(rules.GAME_DIR)): rules.violations(path)
        for path in rules.import_files()
        if rules.violations(path)
    }
    assert not bad, (
        f"{len(bad)} texture(s) hors règle (lancer `cent-ans art models-textures-fix` "
        f"puis réimporter) : {list(bad.items())[:3]}"
    )
