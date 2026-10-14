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
