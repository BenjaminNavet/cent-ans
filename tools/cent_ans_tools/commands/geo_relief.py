"""Groupe ``geo`` (2/2) : relief, pyramide, navgrid, textures, info."""

from __future__ import annotations

from pathlib import Path

import typer
from rich.table import Table

from cent_ans_tools.commands._common import console
from cent_ans_tools.commands._geo_common import (
    _print_sizes,
    _report_navgrid,
)
from cent_ans_tools.commands.geo_map import geo_app


@geo_app.command("relief-all")
def geo_relief_all(
    check: bool = typer.Option(
        False, "--check", help="Liste seulement ce qui manque (code 1 si incomplet)"
    ),
    force: bool = typer.Option(
        False, "--force", help="Recuit toutes les étapes depuis zéro"
    ),
    workers: int = typer.Option(
        0, "--workers", help="Processus (0 = défaut de l'étape)"
    ),
) -> None:
    """Cache complet du relief fin (ADR 0036, lot ZG7b) : pyramid 1-4, detail-dem, hydro-fine, anchors-fine.

    Dans l'ordre, chaque étape reprenant ce qui est déjà sur le disque ;
    relançable après une interruption. Voir docs/geo.md.
    """
    from cent_ans_tools.geo import relief_cache

    report = relief_cache.check()
    for line in report.lines():
        console.print(line)
    plan = report.plan(force)
    commands = {step: command for step, command, _ in relief_cache.STEPS}
    if check:
        if plan:
            console.print(
                "[yellow]Incomplet. Étapes à lancer : "
                + " → ".join(commands[s] for s in plan)
                + "\nCommande unique : uv run --project tools cent-ans geo relief-all[/yellow]"
            )
            raise typer.Exit(1)
        console.print("[green]Cache de relief complet.[/green]")
        return
    if not plan:
        console.print("[green]Cache de relief complet : rien à faire.[/green]")
        return
    result = relief_cache.rebuild(
        force=force, workers=workers or None, log=console.print
    )
    for line in result.report.lines():
        console.print(line)
    if not result.report.complete:
        console.print(
            "[red]Cache encore incomplet (voir ci-dessus) : relancer la commande, "
            "elle reprend où elle s'est arrêtée.[/red]"
        )
        raise typer.Exit(1)
    console.print("[green]Cache de relief complet.[/green]")


@geo_app.command("relief-pack")
def geo_relief_pack(
    out: str = typer.Option(
        "dist/relief", "--out", help="Dossier de sortie (parts + manifeste)"
    ),
) -> None:
    """Empaquette le cache de relief fin en parts « Cent Ans relief » (ADR 0077, lot SZ7).

    N'envoie rien : `geo relief-update` enchaîne recuisson, empaquetage et
    publication (ADR 0149, voir docs/geo.md). Ne pas lancer sur le vrai cache
    (≈ 5 Go) si le disque est presque plein.
    """
    from cent_ans_tools.geo import relief_pack

    result = relief_pack.pack(Path(out))
    console.print(
        f"v{result.version} : {len(result.parts)} part(s), "
        f"{result.total_bytes / 1e9:.2f} Go → {result.out_dir}"
    )
    console.print(f"Manifeste : {result.manifest_path}")


@geo_app.command("relief-update")
def geo_relief_update(
    publish: bool = typer.Option(
        True,
        "--publish/--no-publish",
        help="Publier la Release GitHub du paquet (sinon s'arrêter après l'empaquetage)",
    ),
    out: str = typer.Option(
        "dist/relief", "--out", help="Dossier des parts avant l'envoi"
    ),
    keep_parts: bool = typer.Option(
        False, "--keep-parts", help="Garder les parts après une publication réussie"
    ),
    workers: int = typer.Option(
        0, "--workers", help="Processus (0 = défaut de l'étape)"
    ),
) -> None:
    """Met le paquet de relief à jour avec le code (ADR 0149) : recuit, empaquette, publie.

    Aligne les versions de cuisson du manifeste sur le code, recuit ce qui est
    périmé (puis `geo towns` et `geo landmarks`), et, si la cuisson a changé,
    empaquette et publie la Release `v<N>` (`gh`). Chaque étape est sautée quand
    elle n'a rien à faire ; relançable après une interruption. Reste à commiter
    les fichiers suivis de data/ modifiés. Voir docs/geo.md.
    """
    from cent_ans_tools.geo import relief_update

    try:
        result = relief_update.update(
            out_dir=Path(out),
            do_publish=publish,
            keep_parts=keep_parts,
            workers=workers or None,
            log=console.print,
        )
    except (RuntimeError, OSError, ValueError) as error:
        console.print(f"[red]{error}[/red]")
        raise typer.Exit(1) from error
    if result.packed and not result.published:
        console.print(
            f"[yellow]Paquet v{result.version} prêt dans {out}, non publié "
            "(--no-publish).[/yellow]"
        )
    if result.changed:
        console.print(
            "[yellow]Fichiers suivis modifiés à commiter : git status data/[/yellow]"
        )


@geo_app.command("relief-fetch")
def geo_relief_fetch(
    dest: str = typer.Option(
        "",
        "--dest",
        help="Dossier d'installation (défaut : CENT_ANS_RELIEF_DIR ou data/map)",
    ),
    base_url: str = typer.Option(
        "",
        "--base-url",
        help="URL de base des parts (défaut : data/map/relief_hosting.json)",
    ),
    from_dir: str = typer.Option(
        "",
        "--from-dir",
        help="Installer depuis des parts locales (clé USB, tests) plutôt que par HTTP",
    ),
    if_needed: bool = typer.Option(
        False,
        "--if-needed",
        help="Ne rien faire si le cache installé vaut déjà le paquet publié (ADR 0149)",
    ),
) -> None:
    """Télécharge, vérifie et installe le cache de relief fin (ADR 0077, lot SZ7).

    Reprend les téléchargements interrompus (HTTP Range), refuse toute part dont
    la somme SHA-256 ne correspond pas au manifeste, puis installe atomiquement.
    Lancer ensuite `geo relief-all --check` pour confirmer l'état complet.
    """
    from cent_ans_tools.geo import relief_cache, relief_fetch

    if if_needed:
        needed, reason = relief_fetch.needs_fetch(Path(dest) if dest else None)
        if not needed:
            console.print(f"[green]Relief fin à jour ({reason}).[/green]")
            if relief_fetch.discard_partial_download(Path(dest) if dest else None):
                console.print("Restes d'un téléchargement interrompu supprimés.")
            return
        console.print(f"Relief fin à télécharger : {reason}.")
    result = relief_fetch.fetch(
        dest=Path(dest) if dest else None,
        base_url=base_url or None,
        from_dir=Path(from_dir) if from_dir else None,
        log=console.print,
    )
    console.print(
        f"{result.parts} part(s), {result.total_bytes / 1e9:.2f} Go → {result.pyramid_dir}"
    )
    if (result.dest_dir / relief_cache.MANIFEST).exists():
        report = relief_cache.check(result.dest_dir)
        for line in report.lines():
            console.print(line)
        if not report.complete:
            console.print(
                "[yellow]Cache encore incomplet après installation : compléter avec "
                "uv run --project tools cent-ans geo relief-all[/yellow]"
            )
    else:
        console.print(
            "[yellow]Installé hors du dépôt : lancer le jeu pour vérifier "
            "(avis « relief rapproché »).[/yellow]"
        )


@geo_app.command("detail-check")
def geo_detail_check(
    zones: str = typer.Option(
        "",
        "--zones",
        help="Identifiants de zones séparés par des virgules (toutes sinon)",
    ),
) -> None:
    """E5 comparé à l'ancêtre E4 sur terre, par zone (lot ZG3b) : écart médian et p95.

    Signale (rouge) les zones dont l'écart p95 dépasse
    ``detail_dem.LAND_GAP_ALERT_M`` (5 m) : trait de côte ou relief mal
    raccordé (ex. fuite du rehaussement de rendu, ADR 0036).
    """
    from cent_ans_tools.geo import detail_dem

    zone_ids = tuple(z.strip() for z in zones.split(",") if z.strip())
    for gap in detail_dem.land_gap_report(zone_ids):
        if gap.median_m is None:
            console.print(f"{gap.zone_id:20s} pas de tuiles E5")
            continue
        flag = gap.p95_m > detail_dem.LAND_GAP_ALERT_M
        colour = "red" if flag else "green"
        console.print(
            f"[{colour}]{gap.zone_id:20s} n={gap.n_pixels:8d}  "
            f"médiane={gap.median_m:6.2f} m  p95={gap.p95_m:6.2f} m  "
            f"max={gap.max_m:6.2f} m{'  ALERTE' if flag else ''}[/{colour}]"
        )


@geo_app.command("navgrid")
def geo_navgrid(
    lenient: bool = typer.Option(
        False, "--lenient", help="Écrit la grille même si des colonies sont isolées"
    ),
) -> None:
    """Génère navgrid.png (grille de navigation W/2 × H/2), son aperçu et map.json.navgrid."""
    from cent_ans_tools.geo import navgrid as geo_navgrid_step

    _report_navgrid(geo_navgrid_step.build(strict=not lenient))


@geo_app.command("textures")
def geo_textures(
    force: bool = typer.Option(False, "--force", help="Retélécharge les textures"),
) -> None:
    """Télécharge les textures PBR CC0 (Poly Haven) du terrain de campagne."""
    from cent_ans_tools.geo import textures as geo_textures_step

    paths = geo_textures_step.build(force=force)
    _print_sizes("Textures du terrain", paths)


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
