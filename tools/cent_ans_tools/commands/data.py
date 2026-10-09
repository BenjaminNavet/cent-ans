"""Commandes racine : ``dn-ingest``, ``codex-bundle``, ``export-data``."""

from __future__ import annotations

import json
from pathlib import Path

import typer

from cent_ans_tools import blender, dn_ingest
from cent_ans_tools.commands._common import console

# Commandes enregistrées sur le parseur racine par ``cli.py``.
root_commands = typer.Typer()


@root_commands.command("dn-ingest")
def dn_ingest_command(
    raw: Path = typer.Argument(  # noqa: B008
        ..., exists=True, dir_okay=False, help="Glb brut TRELLIS/SF3D"
    ),
    asset_id: str = typer.Option(..., "--id", help="Identifiant snake_case anglais"),
    asset_class: str = typer.Option(
        ..., "--class", help="Classe (data/art/dn_ingest_classes.json)"
    ),
    length: float | None = typer.Option(None, help="Longueur cible (m)"),
    width: float | None = typer.Option(None, help="Largeur cible (m)"),
    height: float | None = typer.Option(None, help="Hauteur cible (m)"),
    yaw: float = typer.Option(0.0, help="Lacet de correction en degrés (SF3D : 180)"),
    lods: int = typer.Option(3, help="Nombre de LOD (1 à 3)"),
    tex: int | None = typer.Option(None, help="Taille de texture (défaut : classe)"),
    gamma: float | None = typer.Option(
        None, help="Gamma d'albédo forcé (défaut : automatique, fenêtre de luminance)"
    ),
    no_grade: bool = typer.Option(
        False, "--no-grade", help="Pas d'étalonnage de l'albédo"
    ),
    source_image: str | None = typer.Option(None, help="Image source (manifeste)"),
    model_3d: str | None = typer.Option(None, help="Modèle 3D (manifeste)"),
    cost_usd: float | None = typer.Option(None, help="Coût en dollars (manifeste)"),
    generation: Path | None = typer.Option(  # noqa: B008
        None, exists=True, dir_okay=False, help="JSON de provenance (manifeste)"
    ),
    out_dir: Path | None = typer.Option(  # noqa: B008
        None, help="Racine de sortie (défaut game/assets/models/dn)"
    ),
    manifest: Path = typer.Option(  # noqa: B008
        dn_ingest.MANIFEST_PATH, help="Manifeste à mettre à jour"
    ),
) -> None:
    """Transforme un glb brut en asset de jeu (échelle, pivot, LOD, albédo) et l'inscrit au manifeste."""
    classes = dn_ingest.load_classes()
    try:
        job = dn_ingest.build_job(
            asset_id=asset_id,
            asset_class=asset_class,
            raw=raw.resolve(),
            classes=classes,
            out_dir=out_dir,
            length=length,
            width=width,
            height=height,
            yaw_deg=yaw,
            lods=lods,
            tex=tex,
            grade=not no_grade,
            gamma=gamma,
        )
        result = dn_ingest.run_ingest(job, classes)
    except (dn_ingest.IngestError, blender.BlenderError) as error:
        console.print(f"[red]{error}[/red]")
        raise typer.Exit(code=1) from error
    problems = dn_ingest.check_result(job, result, classes)
    entry = dn_ingest.manifest_entry(
        job,
        result,
        asset_class,
        source_image=source_image,
        model_3d=model_3d,
        cost_usd=cost_usd,
        generation=json.loads(generation.read_text()) if generation else None,
    )
    dn_ingest.update_manifest(entry, manifest)
    console.print(
        f"{asset_id} : {result['triangles']} triangles, {result['dimensions_m']} m"
    )
    for problem in problems:
        console.print(f"[yellow]{problem}[/yellow]")
    if problems:
        raise typer.Exit(code=2)


@root_commands.command("codex-bundle")
def codex_bundle_command() -> None:
    """Régénère data/codex_bundle.json (lu par le jeu) depuis data/codex/cdx_*.json."""
    from cent_ans_tools import codex

    data_dir = Path(__file__).resolve().parents[2] / "data"
    target = codex.write_bundle(data_dir)
    console.print(f"Bundle du codex écrit : {target}")


@root_commands.command("export-data")
def export_data_command(
    app_path: str | None = typer.Option(
        None, "--app", help="Application exportée macOS (…/Cent Ans.app)"
    ),
    folder_path: str | None = typer.Option(
        None,
        "--dir",
        help="Dossier de l'export Windows (celui de Cent Ans.exe, ADR 0087)",
    ),
    relief: str = typer.Option(
        "bundle",
        "--relief",
        help="Cache du relief fin : bundle (dans l'app), external (dossier « Cent Ans relief » à côté), none",
    ),
) -> None:
    """Copie data/ et le cache de relief dans un export (tools/export_macos.sh, export_windows.sh)."""
    from pathlib import Path

    from cent_ans_tools import export_data

    if (app_path is None) == (folder_path is None):
        raise typer.BadParameter("indiquer soit --app (macOS), soit --dir (Windows)")
    if app_path is not None:
        result = export_data.stage(Path(app_path) / "Contents" / "Resources", relief)
    else:
        folder = Path(folder_path)
        result = export_data.stage(folder, relief, external_parent=folder)
    console.print(f"data/ : {result.data_bytes / 1e6:.0f} Mo → {result.data_dir}")
    if result.relief_dir is not None:
        console.print(
            f"Relief fin : {result.relief_bytes / 1e9:.2f} Go → {result.relief_dir}"
        )
    else:
        console.print(
            "[yellow]Relief fin non embarqué (zoom rapproché limité).[/yellow]"
        )
    if relief != "none" and not result.report.complete:
        for line in result.report.lines():
            console.print(line)
        console.print(
            "[yellow]Cache de relief incomplet : le jeu exporté affichera l'avis "
            "« relief rapproché limité ». Compléter avec : "
            "uv run --project tools cent-ans geo relief-all[/yellow]"
        )
