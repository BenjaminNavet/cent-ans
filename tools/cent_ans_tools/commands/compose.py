"""Groupe ``compose`` : carte composée façon Total War (lot CC)."""

from __future__ import annotations

from pathlib import Path

import typer

from cent_ans_tools.commands._common import console

compose_app = typer.Typer(
    help="Carte composée : cartogramme, généralisation, planches 2D (lot CC).",
    no_args_is_help=True,
)


@compose_app.command("preview")
def compose_preview(
    strength: list[float] = typer.Option(
        [0.5, 0.7], "--strength", help="Part de la densité des lieux (0..1), répétable"
    ),
    sigma_km: float = typer.Option(
        200.0, help="Lissage gaussien des lieux (km de carte)"
    ),
    sea_density: float = typer.Option(
        1.0, help="Part de la densité moyenne terrestre donnée à la mer"
    ),
    real: bool = typer.Option(True, help="Ajoute la planche de la géographie réelle"),
    width: int = typer.Option(1600, help="Largeur des planches en px"),
    out_dir: Path = typer.Option(
        None, help="Dossier de sortie (défaut ~/dev/cent-ans-raw/compose)"
    ),
) -> None:
    """Planches PNG de la carte composée (une par ``--strength``, plus la réelle)."""
    from cent_ans_tools.compose import preview

    report = preview.make_boards(
        tuple(strength), sigma_km, sea_density, real, width, out_dir or preview.OUT_DIR
    )
    for key, value in report.items():
        if isinstance(value, dict) and "path" in value:
            console.print(
                f"{key} : {value['path']} {value['size']} en {value['total_s']} s"
            )


@compose_app.command("rewrite")
def compose_rewrite(
    dry_run: bool = typer.Option(False, help="Calcule et rapporte sans écrire"),
) -> None:
    """Réécrit les données de jeu selon ``data/map/compose/id_map.json`` (lot C3)."""
    from cent_ans_tools.compose import rewrite

    report = rewrite.run(dry_run=dry_run)
    for line in report.summary_lines():
        console.print(line)
