"""Groupe ``compose`` : prototype de la carte composée (CC, ADR 0355)."""

from __future__ import annotations

import typer

from cent_ans_tools.commands._common import console

compose_app = typer.Typer(
    help="Carte composée façon Total War (prototype).", no_args_is_help=True
)


@compose_app.command("select")
def compose_select(
    provinces: int = typer.Option(180, help="Nombre cible de provinces fusionnées"),
    settlements: int = typer.Option(400, help="Nombre cible de lieux"),
    spacing_km: float = typer.Option(45.0, help="Espacement minimal réel (km)"),
    scope: str = typer.Option(
        "owner",
        help="Fusion entre provinces de même 'owner' ou de même 'realm' (suzerain)",
    ),
    dry_run: bool = typer.Option(False, "--dry-run", help="N'écrit aucun fichier"),
) -> None:
    """Sélectionne les lieux et fusionne les provinces (data/map/compose/)."""
    from cent_ans_tools.compose import select as compose_select_module

    params = compose_select_module.Params(
        province_target=provinces,
        settlement_target=settlements,
        min_spacing_km=spacing_km,
        merge_scope=scope,
    )
    result = compose_select_module.select(params)
    selection = result.selection
    console.print(
        f"{len(selection['provinces'])} provinces, {len(selection['settlements'])} lieux, "
        f"{len(selection['dropped_provinces'])} provinces retirées, "
        f"{len(selection['dropped_factions'])} factions retirées"
    )
    if not dry_run:
        compose_select_module.write(
            result, suffix="" if scope == "owner" else f"_{scope}"
        )
