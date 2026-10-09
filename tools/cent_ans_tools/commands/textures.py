"""Groupe ``textures`` : fabrique de textures locale (TX, ADR 0236)."""

from __future__ import annotations

import typer

textures_app = typer.Typer(
    help="TX : textures régionales générées en local (Z-Image, ADR 0236).",
    no_args_is_help=True,
)


@textures_app.command("families")
def families() -> None:
    """Liste les familles de textures connues."""
    from cent_ans_tools.texture_factory import FAMILIES

    for family in FAMILIES:
        typer.echo(family)
