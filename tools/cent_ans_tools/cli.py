"""Command-line entry point: ``uv run --project tools cent-ans ...``."""

from __future__ import annotations

import typer
from rich.console import Console
from rich.table import Table

from cent_ans_tools import blender, budget, openrouter

app = typer.Typer(help="Outils du projet Cent Ans.", no_args_is_help=True)
budget_app = typer.Typer(
    help="Suivi du budget cloud (docs/budget.md).", no_args_is_help=True
)
models_app = typer.Typer(help="Modèles d'images OpenRouter.", no_args_is_help=True)
blender_app = typer.Typer(help="Scripts Blender.", no_args_is_help=True)
geo_app = typer.Typer(
    help="Carte de campagne : terrain depuis ETOPO / Natural Earth, provinces.",
    no_args_is_help=True,
)
app.add_typer(budget_app, name="budget")
app.add_typer(models_app, name="models")
app.add_typer(blender_app, name="blender")
app.add_typer(geo_app, name="geo")
assets_app = typer.Typer(
    help="Assets du jeu (héraldique, audio, modèles 3D, portraits).",
    no_args_is_help=True,
)
app.add_typer(assets_app, name="assets")

console = Console()


@budget_app.command("show")
def budget_show() -> None:
    """Affiche le tableau des dépenses et le cumul."""
    ledger = budget.BudgetLedger()
    table = Table(title="Budget cloud")
    for column in ("Date", "Service", "Objet", "Estimé", "Réel", "Cumul"):
        table.add_column(column)
    for entry in ledger.entries:
        table.add_row(
            entry.date,
            entry.service,
            entry.subject,
            budget.format_amount(entry.estimated),
            budget.format_amount(entry.actual),
            budget.format_amount(entry.cumulative),
        )
    console.print(table)
    console.print(
        f"Cumul : {budget.format_amount(ledger.total())} / {budget.format_amount(ledger.cap)}"
    )


@budget_app.command("check")
def budget_check(
    amount: float = typer.Argument(..., help="Montant estimé en dollars"),
) -> None:
    """Vérifie qu'une dépense estimée reste sous le plafond (code de sortie 1 sinon)."""
    ledger = budget.BudgetLedger()
    if ledger.check(amount):
        console.print(
            f"[green]OK[/green] : {budget.format_amount(budget.to_money(amount))} tient dans le budget"
        )
    else:
        console.print(
            f"[red]REFUSÉ[/red] : cumul {budget.format_amount(ledger.total())} + {amount} $ > plafond"
        )
        raise typer.Exit(code=1)


@models_app.command("list")
def models_list() -> None:
    """Liste les modèles OpenRouter capables de générer des images (appel gratuit)."""
    table = Table(title="Modèles d'images OpenRouter (USD)")
    table.add_column("Modèle", no_wrap=True)
    table.add_column("Prompt / token")
    table.add_column("Image / token")
    table.add_column("Estimation / image")
    for model in openrouter.list_image_models():
        price = model.estimated_price_per_image()
        table.add_row(
            model.id,
            f"{model.prompt:f}" if model.prompt is not None else "—",
            f"{model.image_output:f}" if model.image_output is not None else "—",
            f"{price:.4f} $" if price is not None else "—",
        )
    console.print(table)


@blender_app.command("smoke")
def blender_smoke() -> None:
    """Lance le script Blender de fumée (cube -> glTF temporaire)."""
    output = blender.smoke()
    if "OK" not in output.splitlines():
        console.print("[red]FAIL[/red] : le script Blender n'a pas imprimé OK")
        raise typer.Exit(code=1)
    console.print("[green]OK[/green] : cube exporté en glTF")


@geo_app.command("build")
def geo_build(
    force: bool = typer.Option(
        False, "--force", help="Retélécharge les données brutes"
    ),
) -> None:
    """Construit data/map/ (terrain puis provinces) et les aperçus docs/img."""
    from cent_ans_tools.geo import build as geo_builder

    result = geo_builder.build(force=force)
    _print_sizes(
        "Sorties du terrain",
        [
            result.map_json,
            result.heightmap,
            result.land_mask,
            result.rivers,
            result.coastline,
            result.preview,
        ],
    )
    _report_provinces(result.provinces)


@geo_app.command("provinces")
def geo_provinces() -> None:
    """Génère provinces.geojson, province_ids.png et docs/img/provinces-preview.png."""
    from cent_ans_tools.geo import provinces as geo_provinces_step

    _report_provinces(geo_provinces_step.build())


def _print_sizes(title: str, paths: list) -> None:
    from cent_ans_tools.geo import build as geo_builder

    table = Table(title=title)
    table.add_column("Fichier")
    table.add_column("Taille", justify="right")
    for path in paths:
        table.add_row(
            str(path.relative_to(geo_builder.REPO_DIR)),
            f"{path.stat().st_size / 1e6:.1f} Mo",
        )
    console.print(table)


def _report_provinces(result) -> None:  # noqa: ANN001
    _print_sizes(
        "Sorties des provinces", [result.geojson, result.id_raster, result.preview]
    )
    counts = list(result.neighbour_counts.values())
    console.print(
        f"{len(counts)} provinces en {result.seconds:.1f} s ; voisins terrestres : "
        f"min {min(counts)}, moyenne {sum(counts) / len(counts):.1f}, max {max(counts)} ; "
        f"liaisons maritimes : {sum(result.sea_neighbour_counts.values()) // 2}"
    )
    if result.snapped_seeds:
        console.print(
            f"[yellow]Seeds déplacés sur la terre[/yellow] : {', '.join(result.snapped_seeds)}"
        )
    if result.snapped_capitals:
        console.print(
            f"[yellow]Capitales rapprochées de leur province[/yellow] : {', '.join(result.snapped_capitals)}"
        )


@geo_app.command("info")
def geo_info() -> None:
    """Affiche les métadonnées de data/map/map.json et l'état des fichiers."""
    from cent_ans_tools.geo import build as geo_builder

    summary = geo_builder.info()
    metadata = summary["metadata"]
    console.print(
        f"CRS : {metadata['crs']}  taille : {metadata['size_px']}  {metadata['meters_per_px']:.1f} m/px"
    )
    console.print(f"Emprise projetée : {metadata['bounds_projected']}")
    console.print(
        f"Altitudes codées : {metadata['height_min_m']} .. {metadata['height_max_m']} m"
    )
    if "height_range_m" in summary:
        low, high = summary["height_range_m"]
        console.print(f"Altitudes présentes : {low:.0f} .. {high:.0f} m")
    if "land_fraction" in summary:
        console.print(f"Fraction de terre : {summary['land_fraction']:.1%}")
    table = Table(title="Fichiers data/map")
    table.add_column("Fichier")
    table.add_column("Taille", justify="right")
    for name, size in summary["files"].items():
        table.add_row(
            name,
            f"{size / 1e6:.1f} Mo" if size is not None else "[yellow]absent[/yellow]",
        )
    console.print(table)


@assets_app.command("heraldry")
def assets_heraldry() -> None:
    """Dessine un écu PNG 128×128 par faction dans game/assets/heraldry/."""
    from cent_ans_tools import heraldry

    paths = heraldry.build()
    console.print(f"[green]OK[/green] : {len(paths)} écus dans {heraldry.HERALDRY_DIR}")


@assets_app.command("menu-art")
def assets_menu_art() -> None:
    """Dessine l'illustration du menu (carte ancienne 2560×1440) dans game/assets/ui/."""
    from cent_ans_tools import menu_art

    path = menu_art.build()
    console.print(
        f"[green]OK[/green] : {path} ({path.stat().st_size / 1e6:.1f} Mo) + {menu_art.SIDECAR_NAME}"
    )


@assets_app.command("audio")
def assets_audio(
    no_music: bool = typer.Option(False, "--no-music", help="Effets seulement"),
) -> None:
    """Synthétise les effets (sfx/) et les 3 musiques modales (music/) en OGG ou WAV."""
    from cent_ans_tools import audio

    paths = audio.build(music=not no_music)
    total = sum(path.stat().st_size for path in paths) / 1e6
    console.print(
        f"[green]OK[/green] : {len(paths)} fichiers ({total:.1f} Mo) dans {audio.AUDIO_DIR}"
    )


@assets_app.command("models")
def assets_models() -> None:
    """Construit les modèles low-poly (Blender headless) dans game/assets/models/*.glb."""
    counts = blender.build_models()
    table = Table(title="Modèles glTF")
    table.add_column("Modèle")
    table.add_column("Triangles", justify="right")
    for name, triangles in counts.items():
        table.add_row(name, str(triangles))
    console.print(table)


@assets_app.command("portraits")
def assets_portraits(
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal de portraits"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : le moins cher retenu)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
) -> None:
    """Génère les portraits manquants (256×256) dans game/assets/portraits/."""
    from decimal import Decimal

    from cent_ans_tools import portraits

    model = model or portraits.DEFAULT_MODEL
    jobs = portraits.plan(limit=limit)
    if dry_run:
        for job in jobs:
            console.rule(job.character_id)
            console.print(job.prompt)
        unit = portraits.KNOWN_PRICES.get(model)
        cost = (
            f"≈ {unit * len(jobs):.2f} $"
            if unit is not None
            else "tarif inconnu hors ligne"
        )
        console.print(
            f"{len(jobs)} portrait(s) avec {model} : {cost} (aucun appel réseau)"
        )
        return
    if not jobs:
        console.print("[green]OK[/green] : tous les portraits existent déjà")
        return
    result = portraits.generate(
        jobs,
        model,
        envelope=Decimal(str(envelope)),
        on_progress=lambda job, spent: console.print(
            f"{job.character_id} ({spent:.4f} $)"
        ),
    )
    console.print(
        f"[green]OK[/green] : {len(result.written)} portrait(s), estimé {result.estimated:.4f} $, réel {result.actual:.4f} $"
    )


if __name__ == "__main__":
    app()
