"""Groupes ``assets ground-materials`` et ``art``."""

from __future__ import annotations

from pathlib import Path

import typer

from cent_ans_tools.commands._common import console
from cent_ans_tools.commands.assets_media import assets_app

art_app = typer.Typer(
    help="Modèles 3D générés : paquet hébergé (ADR 0212).", no_args_is_help=True
)


ground_app = typer.Typer(
    help="HB2 : matières de sol de la carte de campagne (fal.ai, ADR 0143).",
    no_args_is_help=True,
)
assets_app.add_typer(ground_app, name="ground-materials")


@ground_app.command("generate")
def ground_generate(
    only: list[str] = typer.Option(
        None, "--only", help="Identifiants à générer seuls, répétable"
    ),
    attempt: int | None = typer.Option(
        None, "--attempt", help="Reprise (graine + 1000 par reprise)"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche les appels et leur coût, sans appel payant"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """Génère les images brutes manquantes (hors dépôt, jamais payées deux fois)."""
    from cent_ans_tools import ground_materials, local_art

    catalog = ground_materials.load_catalog()
    report = ground_materials.generate(
        catalog, only=only or None, attempt=attempt, dry_run=dry_run, local=local
    )
    verb = "prévus" if dry_run else "faits"
    console.print(
        f"{len(report['planned'])} appel(s) {verb}, {report['cost']:.3f} $ "
        f"({local_art.MODEL_ID if local else catalog['model']})"
    )
    if dry_run:
        for name in report["planned"]:
            console.print(f"  {name}")


@ground_app.command("seamless")
def ground_seamless(
    only: list[str] = typer.Option(
        None, "--only", help="Identifiants à traiter seuls, répétable"
    ),
) -> None:
    """Raccord sans couture, égalisation, normale/rugosité, puis planche 2×2."""
    from cent_ans_tools import ground_materials

    catalog = ground_materials.load_catalog()
    for row in ground_materials.seamless(catalog, only=only or None):
        console.print(
            f"{row['id']:<22} couture {row['seam_raw']:>5} -> {row['seam']:<5} "
            f"taches {row['blotch']}"
        )
    console.print(f"Planche : {ground_materials.board(catalog)}")


@ground_app.command("pack")
def ground_pack() -> None:
    """Empaquette les tuiles en tableaux Texture2DArray + manifeste."""
    from cent_ans_tools import ground_materials

    report = ground_materials.pack(ground_materials.load_catalog())
    console.print(
        f"[green]OK[/green] : {report['layers']} couches, grille {report['grid']}, "
        f"{report['bytes'] / 1e6:.1f} Mo"
    )


@art_app.command("models-pack")
def art_models_pack(
    out: str = typer.Option(
        "dist/models", "--out", help="Dossier de sortie (parts + manifeste)"
    ),
) -> None:
    """Empaquette game/assets/models/dn en parts « Cent Ans modèles » (ADR 0212).

    N'envoie rien : `art models-update` enchaîne empaquetage et publication.
    Incrémente la version de data/art/dn_models_hosting.json si les modèles ont changé.
    """
    from cent_ans_tools import models_package

    result = models_package.pack(Path(out))
    console.print(
        f"v{result.version} : {len(result.parts)} part(s), "
        f"{result.total_bytes / 1e6:.0f} Mo → {result.out_dir}"
    )
    console.print(f"Manifeste : {result.manifest_path}")


@art_app.command("models-fetch")
def art_models_fetch(
    dest: str = typer.Option(
        "",
        "--dest",
        help="Dossier contenant dn/ (défaut : CENT_ANS_MODELS_DIR ou game/assets/models)",
    ),
    base_url: str = typer.Option(
        "", "--base-url", help="URL de base des parts (défaut : dn_models_hosting.json)"
    ),
    from_dir: str = typer.Option(
        "", "--from-dir", help="Installer depuis des parts locales plutôt que par HTTP"
    ),
    if_needed: bool = typer.Option(
        False,
        "--if-needed",
        help="Ne rien faire si les modèles installés valent déjà le paquet publié",
    ),
) -> None:
    """Télécharge, vérifie et installe les modèles générés (ADR 0212).

    Reprend les téléchargements interrompus, refuse toute part dont la somme
    SHA-256 ne correspond pas au manifeste, installe atomiquement.
    """
    from cent_ans_tools import models_package

    destination = Path(dest) if dest else None
    if if_needed:
        needed, reason = models_package.needs_fetch(destination)
        if not needed:
            console.print(f"[green]Modèles générés à jour ({reason}).[/green]")
            if models_package.discard_partial_download(destination):
                console.print("Restes d'un téléchargement interrompu supprimés.")
            return
        console.print(f"Modèles générés à télécharger : {reason}.")
    result = models_package.fetch(
        dest=destination,
        base_url=base_url or None,
        from_dir=Path(from_dir) if from_dir else None,
        log=console.print,
    )
    console.print(
        f"{result.parts} part(s), {result.total_bytes / 1e6:.0f} Mo → {result.models_dir}"
    )


@art_app.command("models-update")
def art_models_update(
    publish: bool = typer.Option(
        True,
        "--publish/--no-publish",
        help="Publier la Release GitHub models-v<N> (sinon s'arrêter après l'empaquetage)",
    ),
    out: str = typer.Option(
        "dist/models", "--out", help="Dossier des parts avant l'envoi"
    ),
    keep_parts: bool = typer.Option(
        False, "--keep-parts", help="Garder les parts après une publication réussie"
    ),
) -> None:
    """Empaquette puis publie les modèles si l'arbre diffère du paquet publié (ADR 0212).

    Ne publier qu'avec l'accord du joueur (dépôt de données public) ; le fichier
    suivi data/art/dn_models_hosting.json est à commiter ensuite.
    """
    from cent_ans_tools import models_package

    try:
        result = models_package.update(
            out_dir=Path(out),
            do_publish=publish,
            keep_parts=keep_parts,
            log=console.print,
        )
    except (ValueError, OSError, RuntimeError) as error:
        console.print(f"[red]{error}[/red]")
        raise typer.Exit(code=1) from error
    if result.changed:
        console.print(
            "Fichier suivi à commiter : data/art/dn_models_hosting.json "
            "(après la publication, avant tout push)."
        )
