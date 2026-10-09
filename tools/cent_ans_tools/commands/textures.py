"""Groupe ``textures`` : fabrique de textures locale (TX, ADR 0236)."""

from __future__ import annotations

from pathlib import Path

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


@textures_app.command("upscale")
def upscale_command(
    family: str = typer.Argument(..., help="Famille (voir `textures families`)."),
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiant d'entrée à traiter (répétable)."
    ),
    method: str = typer.Option(
        "", "--method", help="esrgan, esrgan_light, lanczos (défaut : catalogue)."
    ),
) -> None:
    """Agrandit à 2048 les images brutes retenues (dans `<famille>/upscaled/`)."""
    from cent_ans_tools.texture_factory.catalog import CatalogError, load_catalog
    from cent_ans_tools.texture_factory.upscale import UpscaleError, upscale_family

    try:
        document = load_catalog(family)
        written = upscale_family(document, only=only or None, method=method or None)
    except (CatalogError, UpscaleError) as error:
        typer.echo(f"Erreur : {error}", err=True)
        raise typer.Exit(1) from error
    for path in written:
        typer.echo(str(path))


@textures_app.command("alpha")
def alpha_command(
    family: str = typer.Argument(..., help="Famille (voir `textures families`)."),
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiant d'entrée à traiter (répétable)."
    ),
) -> None:
    """Détoure (rembg local) les entrées `alpha: true` (dans `<famille>/alpha/`)."""
    from cent_ans_tools.texture_factory.alpha import alpha_family
    from cent_ans_tools.texture_factory.catalog import CatalogError, load_catalog

    try:
        document = load_catalog(family)
        written = alpha_family(document, only=only or None)
    except CatalogError as error:
        typer.echo(f"Erreur : {error}", err=True)
        raise typer.Exit(1) from error
    for path in written:
        typer.echo(str(path))


@textures_app.command("micro")
def micro_command(
    family: str = typer.Argument(..., help="Famille (voir `textures families`)."),
    out: Path = typer.Option(..., "--out", help="PNG de sortie."),  # noqa: B008
    size: int = typer.Option(2048, "--size", help="Côté de la tuile (px)."),
    normal: bool = typer.Option(False, "--normal", help="Variante normal map."),
    role: str = typer.Option("", "--role", help="Ne retient que les entrées de ce rôle."),
) -> None:
    """Tuile de micro-détail sans couture (jusqu'à 8 images brutes retenues)."""
    from cent_ans_tools.texture_factory.catalog import CatalogError, load_catalog
    from cent_ans_tools.texture_factory.generate import image_path, load_manifest
    from cent_ans_tools.texture_factory.pbr import micro_detail, micro_detail_normal

    try:
        document = load_catalog(family)
    except CatalogError as error:
        typer.echo(f"Erreur : {error}", err=True)
        raise typer.Exit(1) from error
    manifest = load_manifest(document)
    roles = {entry["id"]: entry["role"] for entry in document["entries"]}
    sources = [
        image_path(document, entry_id, record["attempt"])
        for entry_id, record in sorted(manifest.items())
        if record.get("status") == "ok" and (not role or roles.get(entry_id) == role)
    ][:8]
    if not sources:
        typer.echo("Erreur : aucune image brute retenue pour cette famille.", err=True)
        raise typer.Exit(1)
    build = micro_detail_normal if normal else micro_detail
    typer.echo(str(build(sources, out, size)))


@textures_app.command("pack")
def pack_command(
    family: str = typer.Argument(..., help="Famille (voir `textures families`)."),
    name: list[str] = typer.Option(  # noqa: B008
        None, "--name", help="Paquet du catalogue à produire (répétable)."
    ),
    size: int = typer.Option(
        0, "--size", help="2048 : variante haute dans `hi_dir` (hors dépôt)."
    ),
) -> None:
    """Raccord, PBR et tableaux de textures des paquets d'une famille."""
    from cent_ans_tools.texture_factory.catalog import CatalogError, load_catalog
    from cent_ans_tools.texture_factory.process import build_packs

    try:
        document = load_catalog(family)
        results = build_packs(document, name or None, size=size or None)
    except (CatalogError, ValueError) as error:
        typer.echo(f"Erreur : {error}", err=True)
        raise typer.Exit(1) from error
    for pack_name, result in results.items():
        typer.echo(
            f"{pack_name}\t{result['layers']} couches\tgrille {result['grid']}"
            f"\t{result['bytes'] / 1e6:.1f} Mo"
        )


@textures_app.command("regions")
def regions_command(
    family: str = typer.Argument("building_materials", help="Famille à régions."),
) -> None:
    """Écrit `materials: {rôle: id}` de chaque région dans `data/art/building_regions.json`."""
    from cent_ans_tools.texture_factory.catalog import CatalogError, load_catalog
    from cent_ans_tools.texture_factory.regions import refresh

    try:
        unknown = refresh(load_catalog(family))
    except CatalogError as error:
        typer.echo(f"Erreur : {error}", err=True)
        raise typer.Exit(1) from error
    if unknown:
        typer.echo(f"Régions du catalogue absentes du fichier : {unknown}", err=True)
        raise typer.Exit(1)
    typer.echo("building_regions.json à jour")


@textures_app.command("families")
def families() -> None:
    """Liste les familles de textures connues."""
    from cent_ans_tools.texture_factory import FAMILIES

    for family in FAMILIES:
        typer.echo(family)
