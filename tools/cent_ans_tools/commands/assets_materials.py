"""Groupe ``assets`` (2/2) : matériaux et ornements d'interface."""

from __future__ import annotations

import json
from pathlib import Path

import typer

from cent_ans_tools import budget
from cent_ans_tools.commands._common import console
from cent_ans_tools.commands.assets_media import assets_app


@assets_app.command("materials")
def assets_materials(
    out_dir: Path = typer.Option(
        ...,
        "--out",
        help="Dossier des images brutes et tuiles (hors dépôt, ex. scratch)",
    ),
    only: list[str] = typer.Option(
        None, "--only", help="Identifiants à générer seuls (sonde), répétable"
    ),
    sheet: Path | None = typer.Option(
        None, "--sheet", help="Planche de contrôle PNG à écrire"
    ),
    lot: str | None = typer.Option(
        None,
        "--lot",
        help="Préfixe de ligne et plafond du lot (défaut : GA1 ou le bloc budget)",
    ),
    scans: bool = typer.Option(
        False, "--scans", help="Matières scannées (ambientCG) seules, sans appel payant"
    ),
    build: bool = typer.Option(
        False,
        "--build",
        help="Reconstruire les tableaux des figurines (couches sans tuile : conservées)",
    ),
    layers_sheet: Path | None = typer.Option(
        None, "--layers-sheet", help="Planche JPEG des couches construites (--build)"
    ),
    config: Path | None = typer.Option(
        None,
        "--config",
        help="Fichier de matières (défaut data/art/materials.yaml ; RC5 : water_materials.yaml)",
    ),
    envelope: float | None = typer.Option(
        None,
        "--envelope",
        help="Enveloppe maximale en dollars (abaisse le plafond du bloc budget)",
    ),
    dry_run: bool = typer.Option(
        False,
        "--dry-run",
        help="Affiche les prompts et le coût estimé, sans appel payant",
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """GA/SR/RC5 : matières tuilables (--config), générées ou scannées, + cartes dérivées.

    Une image brute déjà présente dans --out (ou un scan en cache) est réutilisée sans
    appel. --scans --build : scans SR1 + tableaux, couches générées gardées telles quelles.
    """
    from decimal import Decimal

    import numpy as np
    from PIL import Image

    from cent_ans_tools import material_gen

    path = config or material_gen.MATERIALS_PATH
    document = material_gen.load_materials(path)
    entries = {entry["id"]: entry for entry in document["materials"]}
    ids = only or [
        material_id
        for material_id, entry in entries.items()
        if not scans or material_gen.scan_asset_id(entry)
    ]
    unknown = [material_id for material_id in ids if material_id not in entries]
    if unknown:
        raise typer.BadParameter(f"Matière inconnue dans {path} : {', '.join(unknown)}")
    cap = envelope if envelope is None else Decimal(str(envelope))
    if dry_run:
        report = material_gen.plan(ids, out_dir, materials_path=path, local=local)
        for item in report["items"]:
            state = "brute réutilisée" if item["reuse"] else f"{item['cost']} $"
            console.print(f"[bold]{item['id']}[/bold] ({state})\n{item['prompt']}\n")
        limit = report["cap"] if cap is None else min(cap, report["cap"] or cap)
        console.print(
            f"Modèle {report['model']} : {len(report['items'])} matière(s), "
            f"coût estimé {report['total']} $"
            + (f" (plafond {limit} $)" if limit is not None else "")
        )
        if limit is not None and report["total"] > limit:
            console.print("[red]Estimation au-dessus du plafond.[/red]")
            raise typer.Exit(1)
        return
    tiles = {}
    for material_id in ids:
        albedo = material_gen.generate(
            material_id,
            out_dir,
            lot=lot,
            materials_path=path,
            envelope=cap,
            local=local,
        )
        tiles[material_id] = np.asarray(Image.open(albedo).convert("RGB"))
        console.print(f"[green]OK[/green] : {material_id} -> {albedo}")
    if sheet is not None:
        contact = material_gen.contact_sheet(tiles, sheet, params=entries)
        console.print(f"Planche : {contact} ({contact.stat().st_size // 1024} Ko)")
    if build:
        for kind, path in material_gen.build_fine_arrays(out_dir).items():
            console.print(f"Tableau {kind} : {path}")
        if layers_sheet is not None:
            path = material_gen.layers_sheet(layers_sheet)
            console.print(f"Planche : {path} ({path.stat().st_size // 1024} Ko)")
    console.print(f"Cumul de la section : {budget.total()} $")


@assets_app.command("ui-ornaments")
def assets_ui_ornaments(
    dry_run: bool = typer.Option(False, "--dry-run", help="Estime sans appel payant"),
    only: list[str] = typer.Option(
        None, "--only", help="Identifiants à traiter seuls, répétable"
    ),
    sheet: Path | None = typer.Option(
        None, "--sheet", help="Planche avant/après PNG à écrire"
    ),
    install: bool = typer.Option(
        False, "--install", help="Installer les variantes `selected` dans nb/"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """NB : kit d'interface redessiné (data/art/ui_ornaments.yaml)."""
    from cent_ans_tools import ui_ornaments as orn

    config = orn.load_ornaments()
    kit = json.loads((orn.KIT_DIR / "kit.json").read_text(encoding="utf-8"))
    if install:
        selected = {
            entry["id"]: entry["selected"]
            for entry in config["pieces"]
            if "selected" in entry and (not only or entry["id"] in only)
        }
        written = orn.install(selected, kit)
        for path in written:
            console.print(f"[green]OK[/green] : {path}")
        if sheet is not None:
            before = [orn.KIT_DIR / f"{path.stem}.png" for path in written]
            console.print(f"Planche : {orn.contact_sheet(before, written, sheet)}")
        return
    if only:
        config = {
            **config,
            "pieces": [e for e in config["pieces"] if e["id"] in only],
            "decor": [e for e in config["decor"] if e["id"] in only],
        }
    anchor = orn.ANCHOR_PATH.read_bytes() if orn.ANCHOR_PATH.exists() else b""
    requests = orn.build_requests(kit, anchor, config)
    from cent_ans_tools import local_art

    orn.generate(
        requests,
        dry_run=dry_run,
        model=local_art.MODEL_ID if local else config["model"],
    )
    console.print(f"Cumul : {budget.total()} $")
