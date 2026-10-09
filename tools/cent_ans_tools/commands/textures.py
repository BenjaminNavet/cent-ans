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
    max_cost: float = typer.Option(
        2.0,
        "--max-cost",
        help="Refuse le lot fal si son coût estimé dépasse ce montant ($).",
    ),
) -> None:
    """Génère les images brutes d'une famille (reprenable, cache respecté).

    Le coût fal estimé (une passe, sans le cache) s'affiche avant les appels ;
    le consigner ensuite dans `docs/budget.md`.
    """
    from cent_ans_tools.texture_factory.catalog import (
        CatalogError,
        load_catalog,
        select,
    )
    from cent_ans_tools.texture_factory.generate import (
        backend,
        estimate_cost,
        generate,
        image_path,
    )

    try:
        document = load_catalog(family)
        todo = [
            entry
            for entry in select(document, only or None)
            if not image_path(document, entry["id"], 1).is_file()
        ]
        cost = estimate_cost(document, todo)
        typer.echo(
            f"{len(todo)} image(s) à produire ({backend(document)}), coût estimé {cost:.2f} $"
        )
        if cost > max_cost:
            typer.echo(f"Refusé : {cost:.2f} $ > --max-cost {max_cost:.2f} $", err=True)
            raise typer.Exit(1)
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
