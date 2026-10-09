"""Groupe ``textures`` : fabrique de textures locale (TX, ADR 0236)."""

from __future__ import annotations

import typer

textures_app = typer.Typer(
    help="TX : textures régionales générées en local (Z-Image, ADR 0236).",
    no_args_is_help=True,
)


@textures_app.command("generate")
def generate_command(
    family: str = typer.Argument(..., help="Famille (voir `textures families`)."),
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiant d'entrée à produire (répétable)."
    ),
) -> None:
    """Génère les images brutes d'une famille (reprenable, cache respecté)."""
    from cent_ans_tools.texture_factory.catalog import CatalogError, load_catalog
    from cent_ans_tools.texture_factory.generate import generate

    try:
        document = load_catalog(family)
        manifest = generate(document, only=only or None)
    except CatalogError as error:
        typer.echo(f"Erreur : {error}", err=True)
        raise typer.Exit(1) from error
    for entry_id, record in manifest.items():
        typer.echo(f"{entry_id}\t{record['status']}\tessai {record['attempt']}")


@textures_app.command("families")
def families() -> None:
    """Liste les familles de textures connues."""
    from cent_ans_tools.texture_factory import FAMILIES

    for family in FAMILIES:
        typer.echo(family)
