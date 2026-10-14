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


@compose_app.command("editor")
def compose_editor(
    port: int = typer.Option(8766, help="Port local (127.0.0.1)"),
) -> None:
    """Éditeur de carte local (lot CC CE) : crêtes, cols, épingles, lieux, masques. Ouvrir l'adresse affichée."""
    from cent_ans_tools.compose import editor

    server = editor.make_server(editor.Editor(), port)
    console.print(
        f"Éditeur de carte : http://127.0.0.1:{server.server_address[1]}/  (Ctrl+C pour arrêter)"
    )
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        console.print("Arrêt.")


@compose_app.command("rewrite")
def compose_rewrite(
    dry_run: bool = typer.Option(False, help="Calcule et rapporte sans écrire"),
) -> None:
    """Réécrit les données de jeu selon ``data/map/compose/id_map.json`` (lot C3)."""
    from cent_ans_tools.compose import rewrite

    report = rewrite.run(dry_run=dry_run)
    for line in report.summary_lines():
        console.print(line)


@compose_app.command("authored-px")
def compose_authored_px() -> None:
    """Ramène sur le canevas composé les fichiers d'auteur en pixels (coast_types, sea_basins, zones humides, décor)."""
    from cent_ans_tools.compose import authored_px

    console.print_json(data=authored_px.build())


@compose_app.command("c6")
def compose_c6(
    step: str = typer.Argument(..., help="base-supply | rivers | texts | audit"),
    dry_run: bool = typer.Option(False, help="Calcule et rapporte sans écrire"),
) -> None:
    """Recalage C6 de la carte composée : base_supply, km des fleuves, textes (rejouable)."""
    from cent_ans_tools.compose import c6

    if step == "base-supply":
        changes = c6.base_supply(dry_run=dry_run)
        console.print(f"{len(changes)} provinces recalées")
        for province, (before, after) in sorted(
            changes.items(), key=lambda kv: -kv[1][0]
        )[:12]:
            console.print(f"  {province}: {before} -> {after}")
    elif step == "rivers":
        for river, ratio in c6.rivers(dry_run=dry_run):
            console.print(f"  {river}: x{ratio}")
    elif step == "texts":
        console.print(f"fichiers changés : {c6.texts(dry_run=dry_run)}")
    elif step == "audit":
        for folder, count in c6.audit().most_common():
            console.print(f"  {folder}: {count}")
    else:
        raise typer.BadParameter(step)
