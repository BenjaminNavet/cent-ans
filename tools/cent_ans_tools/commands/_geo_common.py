"""Aides d'affichage partagées par les commandes ``geo``."""

from __future__ import annotations

from cent_ans_tools.commands._common import console


def _report_navgrid(result) -> None:  # noqa: ANN001
    _print_sizes("Grille de navigation", [result.path, result.preview])
    console.print(
        f"{result.passable_fraction:.1%} des cases de terre franchissables ; "
        f"passages : {result.crossings_used} ponts et gués, "
        f"{result.road_crossings} croisements route/fleuve "
        f"({result.road_crossings_dropped} écartés, loin de toute colonie), "
        f"{result.passes} cols "
        f"({result.seconds:.0f} s)"
    )
    if result.off_river:
        console.print(
            f"Passages hors du tracé des fleuves : {', '.join(result.off_river)}"
        )
    for warning in result.warnings:
        console.print(f"[yellow]{warning}[/yellow]")


def _print_sizes(title: str, paths: list) -> None:
    from cent_ans_tools.geo import build as geo_builder

    table = Table(title=title)
    table.add_column("Fichier")
    table.add_column("Taille", justify="right")
    for path in paths:
        table.add_row(
            str(path.relative_to(geo_builder.REPO_DIR)),
            f"{path.stat().st_size / 1e6:.1f} Mo",
        )
    console.print(table)


def _report_provinces(result) -> None:  # noqa: ANN001
    _print_sizes(
        "Sorties des provinces", [result.geojson, result.id_raster, result.preview]
    )
    counts = list(result.neighbour_counts.values())
    console.print(
        f"{len(counts)} provinces en {result.seconds:.1f} s ; voisins terrestres : "
        f"min {min(counts)}, moyenne {sum(counts) / len(counts):.1f}, max {max(counts)} ; "
        f"liaisons maritimes : {sum(result.sea_neighbour_counts.values()) // 2}"
    )
    if result.snapped_seeds:
        console.print(
            f"[yellow]Seeds déplacés sur la terre[/yellow] : {', '.join(result.snapped_seeds)}"
        )
    if result.snapped_capitals:
        console.print(
            f"[yellow]Capitales rapprochées de leur province[/yellow] : {', '.join(result.snapped_capitals)}"
        )
