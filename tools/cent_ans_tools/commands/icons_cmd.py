"""Groupe ``icons`` : conversions d'icônes pour le client Bevy."""

from __future__ import annotations

import typer

from cent_ans_tools.commands._common import console

icons_app = typer.Typer(help="Icônes (conversion SVG vers PNG).", no_args_is_help=True)


@icons_app.command("svg-png")
def icons_svg_png() -> None:
    """Rastérise en PNG 128 px à l'encre les icônes SVG seules (skill_*, marqueur de ruines)."""
    from cent_ans_tools import svg_png

    written = svg_png.build()
    console.print(
        f"[green]OK[/green] : {len(written)} icône(s) dans {svg_png.INK_DIR} "
        f"({', '.join(written)})"
    )
