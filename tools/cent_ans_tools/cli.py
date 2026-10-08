"""Command-line entry point: ``uv run --project tools cent-ans ...``."""

from __future__ import annotations

from pathlib import Path

import typer
from rich.console import Console
from rich.table import Table

from cent_ans_tools import blender, budget, openrouter

app = typer.Typer(help="Outils du projet Cent Ans.", no_args_is_help=True)
budget_app = typer.Typer(
    help="Suivi du budget cloud (docs/budget.md).", no_args_is_help=True
)
models_app = typer.Typer(help="Modèles d'images OpenRouter.", no_args_is_help=True)
blender_app = typer.Typer(help="Scripts Blender.", no_args_is_help=True)
geo_app = typer.Typer(
    help="Carte de campagne : terrain depuis ETOPO / Natural Earth, provinces.",
    no_args_is_help=True,
)
app.add_typer(budget_app, name="budget")
app.add_typer(models_app, name="models")
app.add_typer(blender_app, name="blender")
app.add_typer(geo_app, name="geo")
assets_app = typer.Typer(
    help="Assets du jeu (héraldique, audio, modèles 3D, portraits).",
    no_args_is_help=True,
)
app.add_typer(assets_app, name="assets")

console = Console()


@budget_app.command("show")
def budget_show() -> None:
    """Affiche le tableau des dépenses et le cumul."""
    ledger = budget.BudgetLedger()
    table = Table(title="Budget cloud")
    for column in ("Date", "Service", "Objet", "Estimé", "Réel", "Cumul"):
        table.add_column(column)
    for entry in ledger.entries:
        table.add_row(
            entry.date,
            entry.service,
            entry.subject,
            budget.format_amount(entry.estimated),
            budget.format_amount(entry.actual),
            budget.format_amount(entry.cumulative),
        )
    console.print(table)
    console.print(
        f"Cumul : {budget.format_amount(ledger.total())} / {budget.format_amount(ledger.cap)}"
    )


@budget_app.command("check")
def budget_check(
    amount: float = typer.Argument(..., help="Montant estimé en dollars"),
) -> None:
    """Vérifie qu'une dépense estimée reste sous le plafond (code de sortie 1 sinon)."""
    ledger = budget.BudgetLedger()
    if ledger.check(amount):
        console.print(
            f"[green]OK[/green] : {budget.format_amount(budget.to_money(amount))} tient dans le budget"
        )
    else:
        console.print(
            f"[red]REFUSÉ[/red] : cumul {budget.format_amount(ledger.total())} + {amount} $ > plafond"
        )
        raise typer.Exit(code=1)


@models_app.command("list")
def models_list() -> None:
    """Liste les modèles OpenRouter capables de générer des images (appel gratuit)."""
    table = Table(title="Modèles d'images OpenRouter (USD)")
    table.add_column("Modèle", no_wrap=True)
    table.add_column("Prompt / token")
    table.add_column("Image / token")
    table.add_column("Estimation / image")
    for model in openrouter.list_image_models():
        price = model.estimated_price_per_image()
        table.add_row(
            model.id,
            f"{model.prompt:f}" if model.prompt is not None else "—",
            f"{model.image_output:f}" if model.image_output is not None else "—",
            f"{price:.4f} $" if price is not None else "—",
        )
    console.print(table)


@blender_app.command("smoke")
def blender_smoke() -> None:
    """Lance le script Blender de fumée (cube -> glTF temporaire)."""
    output = blender.smoke()
    if "OK" not in output.splitlines():
        console.print("[red]FAIL[/red] : le script Blender n'a pas imprimé OK")
        raise typer.Exit(code=1)
    console.print("[green]OK[/green] : cube exporté en glTF")


@geo_app.command("build")
def geo_build(
    force: bool = typer.Option(
        False, "--force", help="Retélécharge les données brutes"
    ),
) -> None:
    """Construit data/map/ (terrain, provinces, relief 14336 × 12288, routes, colonies, hameaux, grille de navigation) et les aperçus."""
    from cent_ans_tools.geo import build as geo_builder

    result = geo_builder.build(force=force)
    _print_sizes(
        "Sorties du terrain",
        [
            result.map_json,
            result.heightmap,
            result.land_mask,
            result.rivers,
            result.coastline,
            result.preview,
        ],
    )
    _report_provinces(result.provinces)
    if result.relief:
        console.print(
            f"Relief fin : {result.relief.tiles} tuiles, "
            f"{result.relief.total_bytes / 1e6:.1f} Mo"
        )
    if result.roads:
        console.print(
            f"Routes : {result.roads.source}, {result.roads.features} tronçons"
        )
    if result.settlements:
        _report_settlements(result.settlements)
    if result.hamlets:
        console.print(f"Hameaux : {result.hamlets.count}")
    if result.navgrid:
        _report_navgrid(result.navgrid)


@geo_app.command("provinces")
def geo_provinces() -> None:
    """Génère provinces.geojson, province_ids.png et docs/img/provinces-preview.png."""
    from cent_ans_tools.geo import provinces as geo_provinces_step

    _report_provinces(geo_provinces_step.build())


@geo_app.command("settlements")
def geo_settlements() -> None:
    """Projette les colonies et génère settlement_graph.json, settlements_px.json et l'aperçu."""
    from cent_ans_tools.geo import settlements as geo_settlements_step

    _report_settlements(geo_settlements_step.build())


@geo_app.command("roads")
def geo_roads(
    force: bool = typer.Option(False, "--force", help="Retélécharge Itiner-e"),
    computed: bool = typer.Option(
        False, "--computed", help="Force le repli (routes calculées par coût)"
    ),
) -> None:
    """Génère roads.geojson (Itiner-e ou repli) puis remet à jour le graphe des colonies."""
    from cent_ans_tools.geo import roads as geo_roads_step
    from cent_ans_tools.geo import settlements as geo_settlements_step

    result = geo_roads_step.build(force=force, computed=computed)
    _print_sizes("Routes", [result.path])
    console.print(
        f"Source : {result.source} ; {result.features} tronçons, {result.km:.0f} km"
    )
    _report_settlements(geo_settlements_step.build())


@geo_app.command("hamlets")
def geo_hamlets(
    force: bool = typer.Option(False, "--force", help="Retélécharge GeoNames"),
) -> None:
    """Génère hamlets.json (lieux habités GeoNames répartis selon la densité)."""
    from cent_ans_tools.geo import hamlets as geo_hamlets_step

    result = geo_hamlets_step.build(force=force)
    _print_sizes("Hameaux", [result.path])
    console.print(
        f"{result.count} hameaux ({result.candidates} candidats GeoNames), "
        f"{result.provinces} provinces"
    )


@geo_app.command("relief")
def geo_relief(
    force: bool = typer.Option(False, "--force", help="Retélécharge ETOPO"),
) -> None:
    """Relief fin ETOPO seul (28 × 24 tuiles, data/map/height/) : repli ; voir geo relief-shade."""
    from cent_ans_tools.geo import relief as geo_relief_step

    result = geo_relief_step.build(force=force)
    console.print(
        f"{result.tiles} tuiles, {result.total_bytes / 1e6:.1f} Mo au total "
        f"({result.seconds:.0f} s) dans {result.directory}"
    )


def _report_settlements(result) -> None:  # noqa: ANN001
    files = [result.graph, result.positions, result.preview]
    if result.edge_paths is not None:
        files.append(result.edge_paths)
    _print_sizes("Colonies", files)
    console.print(
        f"{result.settlements} colonies ({len(result.fallback_cities)} cités de repli) ; "
        f"arêtes : {result.land_edges} terrestres dont {result.road_edges} sur route "
        f"({result.traced_edges} tracées le long des routes), "
        f"{result.sea_edges} maritimes ; {result.components} composante(s) connexe(s)"
    )
    for warning in result.warnings:
        console.print(f"[yellow]{warning}[/yellow]")


@geo_app.command("splat")
def geo_splat() -> None:
    """Génère splat.png, province_border_dist.png et coast_dist.png (shader du terrain)."""
    from cent_ans_tools.geo import splat as geo_splat_step

    result = geo_splat_step.build()
    _print_sizes(
        "Rasters du shader de terrain",
        [result.splat, result.border_dist, result.coast_dist],
    )
    # Lot R1 : splat.png est ensuite refait avec l'occupation du sol historique.
    _report_landcover()


@geo_app.command("relief-occlusion")
def geo_relief_occlusion(
    exaggeration: float = typer.Option(
        3.0, "--exaggeration", help="Exagération verticale appliquée aux dénivelés"
    ),
    directions: int = typer.Option(16, "--directions", help="Nombre d'azimuts"),
) -> None:
    """Occlusion de vallée (lot RV-D) : relief_occlusion.png, demi-grille de la carte."""
    from cent_ans_tools.geo import relief_occlusion as geo_occlusion_step

    result = geo_occlusion_step.build(exaggeration=exaggeration, directions=directions)
    console.print(f"{result.size[0]}×{result.size[1]} texels, gain {result.gain:.2f}")
    _print_sizes("Occlusion de vallée", [result.path])


@geo_app.command("kk10")
def geo_kk10() -> None:
    """Extrait KK10 (usage anthropique du sol, 1330-1349) par requêtes HTTP partielles.

    Demande h5py et fsspec : uv run --project tools --with h5py --with fsspec
    --with aiohttp --with requests cent-ans geo kk10
    """
    from cent_ans_tools.geo import kk10 as geo_kk10_step

    path = geo_kk10_step.extract()
    _print_sizes("KK10 (cache)", [path])


@geo_app.command("relief-shade")
def geo_relief_shade(
    force: bool = typer.Option(
        False, "--force", help="Recalcule la mosaïque Copernicus (sinon cache .npy)"
    ),
) -> None:
    """Relief fin Copernicus GLO-90 : tuiles fines, heightmap_render.png, relief_shade_<i>.png."""
    from cent_ans_tools.geo import relief_shade as geo_relief_shade_step

    result = geo_relief_shade_step.build(force=force)
    console.print(
        f"{result.tiles} tuiles ({result.tiles_bytes / 1e6:.1f} Mo), "
        f"relief_shade {result.shade_bytes / 1e6:.1f} Mo ({result.seconds:.0f} s)"
    )
    _print_sizes("Relief de rendu", [result.render_heightmap, result.relief_shade])


@geo_app.command("gpu-textures")
def geo_gpu_textures() -> None:
    """Copies GPU (OMR-R2) : relief_shade_bc5_<i>.bin (BC5 + mipmaps), wetlands_bc1_<i>.bin."""
    from cent_ans_tools.geo import block_compress
    from cent_ans_tools.geo import relief_shade as geo_relief_shade_step

    paths = block_compress.build_from_pngs(geo_relief_shade_step.MAP_DIR)
    _print_sizes("Textures GPU", paths)


@geo_app.command("colormap")
def geo_colormap() -> None:
    """Carte de couleur du sol (SS, ADR 0142) : colormap_bc1_<i>.bin (BC1 + mipmaps), aperçu JPEG."""
    from cent_ans_tools.geo import colormap as geo_colormap_step

    paths = geo_colormap_step.build()
    _print_sizes("Carte de couleur du sol", paths)


@geo_app.command("biomes")
def geo_biomes() -> None:
    """Carte des biomes (HB, ADR 0143) : biomes.png (indices 0-7) depuis Köppen-Geiger 1 km."""
    from cent_ans_tools.geo import biomes as geo_biomes_step

    paths = geo_biomes_step.build()
    _print_sizes("Carte des biomes", paths)


@geo_app.command("horizon")
def geo_horizon(
    province: str = typer.Option(
        "",
        "--province",
        help="Ne cuire que ces provinces (liste séparée par des virgules)",
    ),
) -> None:
    """Relief réel autour de chaque province pour l'horizon des batailles (EP2)."""
    from cent_ans_tools.geo import horizon as geo_horizon_step

    only = [p.strip() for p in province.split(",") if p.strip()]
    result = geo_horizon_step.build(only=only or None)
    console.print(
        f"{result.tiles} tuiles d'horizon ({result.total_bytes / 1e6:.1f} Mo, "
        f"{result.seconds:.0f} s) dans game/assets/horizon/relief/"
    )


@geo_app.command("battle-site")
def geo_battle_site(
    maps: str = typer.Option(
        "",
        "--maps",
        help="Ne cuire que ces cartes (ids séparés par des virgules)",
    ),
    no_tiles: bool = typer.Option(
        False, "--no-tiles", help="Relief du champ seulement, sans tuile d'horizon"
    ),
) -> None:
    """Relief réel des champs de bataille historiques et tuile d'horizon du site (EP7)."""
    from cent_ans_tools.geo import battle_site

    only = [m.strip() for m in maps.split(",") if m.strip()]
    done = battle_site.bake(only or None, tiles=not no_tiles)
    console.print(f"Sites cuits : {', '.join(done) or 'aucun'}")


@app.command("export-data")
def export_data_command(
    app_path: str | None = typer.Option(
        None, "--app", help="Application exportée macOS (…/Cent Ans.app)"
    ),
    folder_path: str | None = typer.Option(
        None,
        "--dir",
        help="Dossier de l'export Windows (celui de Cent Ans.exe, ADR 0087)",
    ),
    relief: str = typer.Option(
        "bundle",
        "--relief",
        help="Cache du relief fin : bundle (dans l'app), external (dossier « Cent Ans relief » à côté), none",
    ),
) -> None:
    """Copie data/ et le cache de relief dans un export (tools/export_macos.sh, export_windows.sh)."""
    from pathlib import Path

    from cent_ans_tools import export_data

    if (app_path is None) == (folder_path is None):
        raise typer.BadParameter("indiquer soit --app (macOS), soit --dir (Windows)")
    if app_path is not None:
        result = export_data.stage(Path(app_path) / "Contents" / "Resources", relief)
    else:
        folder = Path(folder_path)
        result = export_data.stage(folder, relief, external_parent=folder)
    console.print(f"data/ : {result.data_bytes / 1e6:.0f} Mo → {result.data_dir}")
    if result.relief_dir is not None:
        console.print(
            f"Relief fin : {result.relief_bytes / 1e9:.2f} Go → {result.relief_dir}"
        )
    else:
        console.print(
            "[yellow]Relief fin non embarqué (zoom rapproché limité).[/yellow]"
        )
    if relief != "none" and not result.report.complete:
        for line in result.report.lines():
            console.print(line)
        console.print(
            "[yellow]Cache de relief incomplet : le jeu exporté affichera l'avis "
            "« relief rapproché limité ». Compléter avec : "
            "uv run --project tools cent-ans geo relief-all[/yellow]"
        )


@geo_app.command("pyramid")
def geo_pyramid(
    levels: str = typer.Option(
        "1,2,3,4", "--levels", help="Étages à cuire parmi 1-4 (ex. « 1,2 »)"
    ),
    force: bool = typer.Option(
        False, "--force", help="Réécrit les tuiles déjà présentes dans le cache"
    ),
    workers: int = typer.Option(0, "--workers", help="Processus (0 = tous les cœurs)"),
    limit: int = typer.Option(
        0, "--limit", help="Au plus N blocs par palier (essais ; 0 = tout)"
    ),
) -> None:
    """Pyramide de relief E1-E4 (GLO-90 puis GLO-30 corrigé) et manifeste (ADR 0036)."""
    from cent_ans_tools.geo import pyramid as geo_pyramid_step

    wanted = tuple(sorted({int(part) for part in levels.split(",") if part.strip()}))
    if not wanted or any(level not in (1, 2, 3, 4) for level in wanted):
        raise typer.BadParameter("--levels : étages 1 à 4")
    result = geo_pyramid_step.build(
        levels=wanted, force=force, workers=workers or None, limit=limit or None
    )
    counts = ", ".join(f"E{k} {n}" for k, n in sorted(result.per_level.items()))
    console.print(
        f"{result.tiles_written} tuiles écrites ({counts or 'aucune'}), "
        f"{result.total_bytes / 1e6:.1f} Mo, {result.skipped_units} blocs déjà faits "
        f"({result.seconds:.0f} s)"
    )


@geo_app.command("landcover")
def geo_landcover() -> None:
    """Occupation du sol vers 1340 : splat.png (forêts KK10 + massifs nommés), wetlands.png, forest_kind.png."""
    _report_landcover()


def _report_landcover() -> None:
    from cent_ans_tools.geo import landcover as geo_landcover_step

    result = geo_landcover_step.build()
    _print_sizes(
        "Occupation du sol (1340)",
        [result.splat, result.wetlands, result.forest_kind],
    )
    console.print(
        f"forêt {result.forest_share:.1%} des terres, défriché (KK10) {result.cleared_share:.1%}"
    )


@geo_app.command("rivers-render")
def geo_rivers_render(
    fine_min_order: int = typer.Option(
        0,
        "--fine-min-order",
        help="Verse les rivières du réseau fin (pyramide hydro_fine) d'ordre de Strahler "
        ">= N (5 conseillé) ; 0 = Natural Earth seul",
    ),
    fine_min_length_km: float = typer.Option(
        15.0,
        "--fine-min-length-km",
        help="Longueur minimale (km) d'une rivière fine versée",
    ),
) -> None:
    """Génère rivers_render.json, river_bed.png et crossings_px.json (rendu des fleuves, V4)."""
    from cent_ans_tools.geo import river_render

    fine = (
        river_render.FineOptions(fine_min_order, fine_min_length_km)
        if fine_min_order > 0
        else None
    )
    result = river_render.build(fine=fine)
    if fine is not None and result.fine_missing:
        console.print(
            f"[yellow]Réseau fin introuvable ({result.fine_missing}) : rivières fines "
            "ignorées, rendu depuis Natural Earth seul. Lancer d'abord "
            "`cent-ans geo hydro-fine` sur une machine qui a la pyramide.[/yellow]"
        )
    elif fine is not None:
        console.print(
            f"Réseau fin : {result.fine_added} tronçons versés (ordre >= "
            f"{fine.min_order}, >= {fine.min_length_km:g} km), "
            f"{result.fine_dropped} doublons de Natural Earth écartés"
        )
    _print_sizes("Rendu des fleuves", [result.render, result.bed, result.crossings])
    console.print(
        f"{result.rivers} tronçons, {result.points} points ; "
        f"{result.snapped} passages recalés sur leur fleuve"
    )
    if result.unsnapped:
        console.print(
            f"[yellow]Hors fleuve affiché : {', '.join(result.unsnapped)}[/yellow]"
        )


@geo_app.command("sea-lanes")
def geo_sea_lanes() -> None:
    """Trace les routes maritimes sur l'eau : sea_lanes_px.json (lot SL1)."""
    from cent_ans_tools.geo import sea_lanes

    result = sea_lanes.build()
    _print_sizes("Routes maritimes", [result.output])
    console.print(f"{result.lanes} routes, {result.total_km:.0f} km au total")
    for warning in result.warnings:
        console.print(f"[yellow]{warning}[/yellow]")


@geo_app.command("land-mask")
def geo_land_mask() -> None:
    """Masque terre de 1340 : sans retenues modernes, avec les lacs historiques (LR-10)."""
    from cent_ans_tools.geo import historical_water

    result = historical_water.build()
    _print_sizes("Masque terre", [result.land_mask])
    console.print(
        f"{result.to_land} px redevenus terre, {result.to_water} px devenus eau, "
        f"{result.refilled_provinces} px rendus à une province ; retenues retirées : "
        f"{', '.join(result.dropped_reservoirs) or 'aucune'}"
    )
    console.print(
        "Lacs historiques creusés (px) : "
        + ", ".join(f"{key} {value}" for key, value in result.punched.items())
    )
    if result.settlements_in_water:
        console.print(
            f"[yellow]Colonies passées dans l'eau : "
            f"{', '.join(result.settlements_in_water)}[/yellow]"
        )
    console.print("Ensuite : cent-ans geo lakes, puis cent-ans geo navgrid.")


@geo_app.command("lakes")
def geo_lakes(
    min_area_px: int = typer.Option(
        8, "--min-area-px", help="Surface minimale d'un lac (px carte)"
    ),
    simplify_px: float = typer.Option(
        0.6, "--simplify-px", help="Tolérance de simplification du contour (px)"
    ),
    natural_earth: str = typer.Option(
        "", "--natural-earth", help="ne_10m_lakes.shp (noms), défaut : tools/geo/raw"
    ),
) -> None:
    """Contours et niveaux des lacs : data/map/lakes.json (lot SS3)."""
    from cent_ans_tools.geo import lakes

    params = lakes.LakesParams(min_area_px=min_area_px, simplify_px=simplify_px)
    result = lakes.build(
        params=params,
        natural_earth=Path(natural_earth)
        if natural_earth
        else lakes.NATURAL_EARTH_LAKES,
    )
    _print_sizes("Lacs", [result.output])
    console.print(
        f"{result.lakes} lacs ({result.named} nommés), {result.vertices} sommets, "
        f"{result.below_sea} sous le niveau de la mer et {result.not_flat} non plats ignorés, "
        f"retenues exclues : {', '.join(result.excluded_reservoirs) or 'aucune'}, "
        f"lacs historiques : {', '.join(result.historical) or 'aucun'}"
    )


@geo_app.command("detail-dem")
def geo_detail_dem(
    zones: str = typer.Option(
        "",
        "--zones",
        help="Identifiants de zones séparés par des virgules (toutes sinon)",
    ),
    force: bool = typer.Option(
        False, "--force", help="Recuit les zones même si elles sont à jour"
    ),
) -> None:
    """Relief palier 3 (E5-E7, 11 → 2,8 m) sur les zones de détail (ADR 0036, lot ZG3)."""
    from cent_ans_tools.geo import detail_dem

    zone_ids = tuple(z.strip() for z in zones.split(",") if z.strip())
    result = detail_dem.build(zone_ids, force=force, log=console.print)
    for level in sorted(result.tiles):
        console.print(
            f"E{level} : {result.tiles[level]} tuiles, "
            f"{result.bytes_by_level[level] / 1e6:.1f} Mo"
        )
    total = sum(result.bytes_by_level.values())
    console.print(
        f"Palier 3 : {total / 1e9:.2f} Go de tuiles, bruts {result.raw_bytes / 1e9:.2f} Go, "
        f"{result.seconds:.0f} s"
    )
    for note in result.notes:
        console.print(f"[yellow]{note}[/yellow]")


@geo_app.command("hydro-fine")
def geo_hydro_fine(
    workers: int = typer.Option(0, "--workers", help="Processus parallèles (0 = auto)"),
    sources: str = typer.Option(
        "topage,osor,euhydro,naturalearth",
        "--sources",
        help="Sources à traiter, séparées par des virgules",
    ),
) -> None:
    """Réseau hydrographique fin recalé sur la pyramide de relief (ADR 0036, lot ZG5a)."""
    from cent_ans_tools.geo import hydro_fine

    chosen = tuple(s.strip() for s in sources.split(",") if s.strip())
    result = hydro_fine.build(
        workers=workers or None, sources=chosen, log=console.print
    )
    for source, km in result.per_source_km.items():
        console.print(f"{source} : {km:.0f} km")
    console.print(
        f"{result.lines} lignes, {result.points} points, {result.tiles} tuiles E2, "
        f"{result.total_bytes / 1e6:.1f} Mo, {result.seconds:.0f} s → {result.manifest}"
    )


@geo_app.command("anchors-fine")
def geo_anchors_fine(
    workers: int = typer.Option(0, "--workers", help="Processus parallèles (0 = auto)"),
) -> None:
    """Colonies, hameaux, ponts et routes recalés sur le relief fin (ADR 0036, lot ZG5a)."""
    from cent_ans_tools.geo import fine_anchors

    result = fine_anchors.build(workers=workers or None, log=console.print)
    console.print(result.summary())


@geo_app.command("towns")
def geo_towns() -> None:
    """Emprise des villes ordinaires vers 1340 (ADR 0036, lot ZG6) : data/map/towns_1340.json."""
    from cent_ans_tools.geo import towns

    result = towns.build(log=console.print)
    console.print(result.summary())


@geo_app.command("landmarks")
def geo_landmarks(
    city: list[str] = typer.Option(  # noqa: B008
        None, "--city", help="Identifiant de ville (répétable)"
    ),
    refresh_osm: bool = typer.Option(
        False, "--refresh-osm", help="Retélécharge l'extrait OpenStreetMap (Overpass)"
    ),
) -> None:
    """Villes emblématiques 1:1 (ADR 0078, lot VH4) : rues OSM et fleuve fin de data/landmarks_v2/."""
    from cent_ans_tools.geo import landmarks_v2

    result = landmarks_v2.build(
        only=city or None, refresh_osm=refresh_osm, log=console.print
    )
    console.print(result.summary())


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
    from pathlib import Path

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
    from pathlib import Path

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
    from pathlib import Path

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


@geo_app.command("textures")
def geo_textures(
    force: bool = typer.Option(False, "--force", help="Retélécharge les textures"),
) -> None:
    """Télécharge les textures PBR CC0 (Poly Haven) du terrain de campagne."""
    from cent_ans_tools.geo import textures as geo_textures_step

    paths = geo_textures_step.build(force=force)
    _print_sizes("Textures du terrain", paths)


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


@assets_app.command("heraldry")
def assets_heraldry() -> None:
    """Dessine un écu PNG 128×128 par faction dans game/assets/heraldry/."""
    from cent_ans_tools import heraldry

    paths = heraldry.build()
    console.print(f"[green]OK[/green] : {len(paths)} écus dans {heraldry.HERALDRY_DIR}")
    houses = heraldry.build_houses()
    console.print(
        f"[green]OK[/green] : {len(houses)} écus de maison dans {heraldry.HOUSES_DIR}"
    )


@assets_app.command("banners")
def assets_banners() -> None:
    """Dessine bannières (256×512), pennons (512×128) et étendards (1024×256) de chaque faction."""
    from cent_ans_tools import banners

    paths = banners.build()
    console.print(f"[green]OK[/green] : {len(paths)} images dans {banners.BANNERS_DIR}")


@assets_app.command("icons")
def assets_icons(
    offline: bool = typer.Option(
        False, "--offline", help="Cache local seulement, aucun accès réseau"
    ),
) -> None:
    """Icônes game-icons.net (CC BY 3.0) teintées encre sépia dans game/assets/icons/."""
    from cent_ans_tools import icons

    missing = icons.missing_icons()
    if missing:
        console.print(f"[red]Icônes manquantes[/red] : {', '.join(missing)}")
        raise typer.Exit(code=1)
    rows, written = icons.build(offline=offline)
    files = len({row.file for row in rows})
    console.print(
        f"[green]OK[/green] : {len(rows)} identifiants, {files} SVG, "
        f"{written} fichier(s) écrit(s) dans {icons.ICONS_DIR}"
    )


@assets_app.command("ink-icons")
def assets_ink_icons(
    kind: str = typer.Option(
        "all", "--kind", help="icon, medallion ou all (les deux, icônes d'abord)"
    ),
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiants à générer seuls (sonde), répétable"
    ),
    limit: int | None = typer.Option(None, "--limit", help="Nombre maximal d'images"),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    build_only: bool = typer.Option(
        False, "--build-only", help="Aucune génération : dérive les PNG des sources"
    ),
    envelope: float | None = typer.Option(
        None, "--envelope", help="Enveloppe (défaut : reste du plafond du lot)"
    ),
    group: str | None = typer.Option(
        None,
        "--group",
        help='Ne traite que les entrées de ce groupe du catalogue (ex. "trait", DA7c)',
    ),
    subject: str | None = typer.Option(
        None,
        "--subject",
        help="Préfixe de ligne du grand livre et de plafond (défaut : le lot DA5)",
    ),
    budget_cap: float | None = typer.Option(
        None,
        "--budget-cap",
        help="Plafond du lot en dollars (défaut : budget_cap_usd du catalogue, lot DA5)",
    ),
) -> None:
    """DA5 : icônes d'action à l'encre et boutons-médaillons (une image par icône).

    Le même catalogue et le même outil couvrent d'autres lots (DA7c : icônes de trait,
    groupe ``trait``) via ``--group``/``--subject``/``--budget-cap``, avec leur propre
    enveloppe et leur propre ligne de grand livre plutôt que le plafond DA5 déjà dépensé.
    """
    from decimal import Decimal

    from cent_ans_tools import ink_icons
    from cent_ans_tools.budget import BudgetLedger

    catalog = ink_icons.load_catalog()
    model = catalog["model"]
    budget_session = catalog.get("budget_session")
    subject_line = subject or ink_icons.BUDGET_SUBJECT
    prefix = subject_line.split(" :")[0] + " :"
    lot_label = prefix.rstrip(" :")
    if not build_only:
        cap = (
            Decimal(str(budget_cap))
            if budget_cap is not None
            else Decimal(str(catalog["budget_cap_usd"]))
        )
        spent = ink_icons.lot_spent(BudgetLedger(), prefix)
        remaining = cap - spent
        if envelope is not None:
            remaining = min(remaining, Decimal(str(envelope)))
        console.print(
            f"Lot {lot_label} : {spent:.2f} $ déjà dépensés, enveloppe {remaining:.2f} $"
        )
        kinds = ["icon", "medallion"] if kind == "all" else [kind]
        for current in kinds:
            jobs = ink_icons.plan(
                catalog, kind=current, group=group, only=only or None, limit=limit
            )
            convert = (
                ink_icons.to_raw_icon
                if current == "icon"
                else ink_icons.to_raw_medallion
            )
            before = ink_icons.lot_spent(BudgetLedger(), prefix)
            _run_art_batch(
                jobs,
                model,
                float(remaining),
                dry_run,
                f"image(s) ({current})",
                subject_line,
                convert,
                budget_session=budget_session,
            )
            remaining -= ink_icons.lot_spent(BudgetLedger(), prefix) - before
        if dry_run:
            return
    report = ink_icons.build(catalog)
    sheet = ink_icons.contact_sheet(catalog)
    console.print(
        f"[green]OK[/green] : {len(report['icons'])} icône(s), "
        f"{len(report['medallions'])} médaillon(s), "
        f"{len(report.get('cursors', []))} curseur(s) ; planche {sheet}"
    )
    if report["missing"]:
        console.print(f"[yellow]Sans source[/yellow] : {', '.join(report['missing'])}")


@assets_app.command("entity-icons")
def assets_entity_icons(
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiants à générer seuls (sonde), répétable"
    ),
    limit: int | None = typer.Option(None, "--limit", help="Nombre maximal d'images"),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    build_only: bool = typer.Option(
        False,
        "--build-only",
        help="Aucune génération : dérive et encadre les miniatures",
    ),
    envelope: float | None = typer.Option(
        None, "--envelope", help="Enveloppe (défaut : reste du plafond du lot DA5b)"
    ),
) -> None:
    """DA5b : icônes d'entité en miniatures peintes (dérivées des illustrations, sinon générées)."""
    from decimal import Decimal

    from cent_ans_tools import entity_icons
    from cent_ans_tools.budget import BudgetLedger

    catalog = entity_icons.load_catalog()
    if not build_only:
        spent = entity_icons.lot_spent(BudgetLedger())
        remaining = Decimal(str(catalog["budget_cap_usd"])) - spent
        if envelope is not None:
            remaining = min(remaining, Decimal(str(envelope)))
        console.print(
            f"Lot DA5b : {spent:.2f} $ déjà dépensés, enveloppe {remaining:.2f} $"
        )
        jobs = entity_icons.plan(catalog, only=only or None, limit=limit)
        _run_art_batch(
            jobs,
            catalog["model"],
            float(remaining),
            dry_run,
            "miniature(s)",
            entity_icons.BUDGET_SUBJECT,
            entity_icons.to_raw,
        )
        if dry_run:
            return
    report = entity_icons.build(catalog)
    sheet = entity_icons.contact_sheet(catalog)
    console.print(
        f"[green]OK[/green] : {len(report['derived'])} dérivée(s), "
        f"{len(report['generated'])} générée(s) ; planche {sheet}"
    )
    if report["missing"]:
        console.print(f"[yellow]Sans source[/yellow] : {', '.join(report['missing'])}")


@assets_app.command("unit-emblems")
def assets_unit_emblems() -> None:
    """OMR R5 : sources locales (emblème doré sur azur) des miniatures d'unités sans illustration."""
    from cent_ans_tools import unit_emblems

    paths = unit_emblems.build()
    console.print(
        f"[green]OK[/green] : {len(paths)} emblème(s) dans {unit_emblems.OUT_DIR} ; "
        "encadrer ensuite avec « assets entity-icons --build-only »"
    )


@assets_app.command("menu-art")
def assets_menu_art() -> None:
    """Dessine l'illustration du menu (carte ancienne 2560×1440) dans game/assets/ui/."""
    from cent_ans_tools import menu_art

    path = menu_art.build()
    console.print(
        f"[green]OK[/green] : {path} ({path.stat().st_size / 1e6:.1f} Mo) + {menu_art.SIDECAR_NAME}"
    )


@assets_app.command("ui-illumination")
def assets_ui_illumination() -> None:
    """Peint le kit d'UI enluminé (textures 9-slice) dans game/assets/ui/illumination/."""
    from cent_ans_tools import ui_illumination

    paths = ui_illumination.build()
    console.print(
        f"[green]OK[/green] : {len(paths)} texture(s) dans {ui_illumination.OUTPUT_DIR}"
    )


@assets_app.command("audio")
def assets_audio(
    no_music: bool = typer.Option(False, "--no-music", help="Effets seulement"),
) -> None:
    """Synthétise les effets (sfx/) et les 3 musiques modales (music/) en OGG ou WAV."""
    from cent_ans_tools import audio

    paths = audio.build(music=not no_music)
    total = sum(path.stat().st_size for path in paths) / 1e6
    console.print(
        f"[green]OK[/green] : {len(paths)} fichiers ({total:.1f} Mo) dans {audio.AUDIO_DIR}"
    )


@assets_app.command("models")
def assets_models() -> None:
    """Construit les modèles low-poly (Blender headless) dans game/assets/models/*.glb."""
    counts = blender.build_models()
    table = Table(title="Modèles glTF")
    table.add_column("Modèle")
    table.add_column("Triangles", justify="right")
    for name, triangles in counts.items():
        table.add_row(name, str(triangles))
    console.print(table)


@assets_app.command("portraits")
def assets_portraits(
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal de portraits"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : le moins cher retenu)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
) -> None:
    """Génère les portraits manquants (256×256) dans game/assets/portraits/."""
    from decimal import Decimal

    from cent_ans_tools import portraits

    model = model or portraits.DEFAULT_MODEL
    jobs = portraits.plan(limit=limit)
    if dry_run:
        for job in jobs:
            console.rule(job.character_id)
            console.print(job.prompt)
        unit = portraits.KNOWN_PRICES.get(model)
        cost = (
            f"≈ {unit * len(jobs):.2f} $"
            if unit is not None
            else "tarif inconnu hors ligne"
        )
        console.print(
            f"{len(jobs)} portrait(s) avec {model} : {cost} (aucun appel réseau)"
        )
        return
    if not jobs:
        console.print("[green]OK[/green] : tous les portraits existent déjà")
        return
    result = portraits.generate(
        jobs,
        model,
        envelope=Decimal(str(envelope)),
        on_progress=lambda job, spent: console.print(
            f"{job.character_id} ({spent:.4f} $)"
        ),
    )
    console.print(
        f"[green]OK[/green] : {len(result.written)} portrait(s), estimé {result.estimated:.4f} $, réel {result.actual:.4f} $"
    )


@assets_app.command("portrait-archetypes")
def assets_portrait_archetypes(
    limit: int | None = typer.Option(None, "--limit", help="Nombre maximal d'images"),
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Clés à générer seules (sonde), répétable"
    ),
    no_aged: bool = typer.Option(False, "--no-aged", help="Archétypes seulement"),
    no_archetypes: bool = typer.Option(
        False, "--no-archetypes", help="Variantes âgées seulement"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        8.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
) -> None:
    """DA2 : archétypes de portraits et variantes âgées (512×512 JPEG)."""
    from cent_ans_tools import portrait_archetypes, portraits

    model = model or portraits.DEFAULT_MODEL
    jobs = portrait_archetypes.plan(
        archetypes=not no_archetypes,
        aged=not no_aged,
        only=only or None,
        limit=limit,
    )
    _run_art_batch(
        jobs,
        model,
        envelope,
        dry_run,
        "portrait(s) vivant(s)",
        "DA2 : portraits vivants (archétypes et variantes âgées)",
        portrait_archetypes.to_archetype_jpg,
    )


def _run_art_batch(
    jobs, model, envelope, dry_run, noun, subject, convert, budget_session=None
) -> None:
    """Dry-run listing or paid batch for an image job list (event art, illustrations).

    ``budget_session`` names the `docs/budget.md` heading that funds this batch, when it is
    not necessarily the last table in the file (see `portraits.generate`).
    """
    from decimal import Decimal

    from cent_ans_tools import portraits

    if dry_run:
        for job in jobs:
            console.rule(job.character_id)
            console.print(job.prompt)
        unit = portraits.KNOWN_PRICES.get(model)
        cost = (
            f"≈ {unit * len(jobs):.2f} $"
            if unit is not None
            else "tarif inconnu hors ligne"
        )
        console.print(f"{len(jobs)} {noun} avec {model} : {cost} (aucun appel réseau)")
        return
    if not jobs:
        console.print(f"[green]OK[/green] : aucune {noun} manquante")
        return
    result = portraits.generate(
        jobs,
        model,
        envelope=Decimal(str(envelope)),
        budget_session=budget_session,
        subject=f"{subject} ({len(jobs)} × {model})",
        convert=convert,
        on_progress=lambda job, spent: console.print(
            f"{job.character_id} ({spent:.4f} $)"
        ),
    )
    console.print(
        f"[green]OK[/green] : {len(result.written)} {noun}, estimé {result.estimated:.4f} $, réel {result.actual:.4f} $"
    )


@assets_app.command("event-art")
def assets_event_art(
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'illustrations"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
) -> None:
    """Génère les miniatures d'événements manquantes (768×432) dans game/assets/events/."""
    from cent_ans_tools import event_art, portraits

    _run_art_batch(
        event_art.plan(limit=limit),
        model or portraits.DEFAULT_MODEL,
        envelope,
        dry_run,
        "miniature(s)",
        "Miniatures d'événements",
        event_art.to_miniature_jpg,
    )


@assets_app.command("illustrations")
def assets_illustrations(
    category: str | None = typer.Option(
        None,
        "--category",
        help="unit_types, buildings, technologies, factions, séparées par des virgules (défaut : toutes)",
    ),
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'illustrations"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """Génère les miniatures de l'encyclopédie (640×360) dans game/assets/illustrations/."""
    from cent_ans_tools import entry_art, local_art, portraits

    categories = (
        tuple(part.strip() for part in category.split(","))
        if category
        else entry_art.CATEGORIES
    )
    jobs = entry_art.plan(categories=categories, limit=limit)
    if local:
        typer.echo(f"{len(jobs)} illustration(s) à générer localement")
        if dry_run:
            return
        written, failed = local_art.generate(
            jobs,
            entry_art.convert,
            on_progress=lambda job: typer.echo(f"  {job.out_path.name}"),
        )
        typer.echo(f"{len(written)} écrite(s), {len(failed)} échec(s)")
        for job, reason in failed:
            typer.echo(f"  échec {job.character_id} : {reason}", err=True)
        if failed:
            raise typer.Exit(1)
        return
    _run_art_batch(
        jobs,
        model or portraits.DEFAULT_MODEL,
        envelope,
        dry_run,
        "illustration(s)",
        "Illustrations de l'encyclopédie",
        entry_art.convert,
    )


@assets_app.command("horizon-panoramas")
def assets_horizon_panoramas(
    ids: str = typer.Option(
        "", "--ids", help="Panoramas à générer (virgules ; défaut : tous)"
    ),
    generate: bool = typer.Option(
        False, "--generate", help="Appels payants OpenRouter (sinon traitement seul)"
    ),
    dry_run: bool = typer.Option(False, "--dry-run", help="Affiche les invites"),
) -> None:
    """Panoramas d'horizon peints des batailles (EP2) : génération puis détourage."""
    from cent_ans_tools import horizon_panoramas

    data = horizon_panoramas.load_data()
    wanted = [i.strip() for i in ids.split(",") if i.strip()] or list(data["panoramas"])
    if generate or dry_run:
        horizon_panoramas.generate(wanted, dry_run=dry_run, data=data)
    if not dry_run:
        meta = horizon_panoramas.process_all(data)
        console.print(f"{len(meta['panoramas'])} panoramas traités")


@assets_app.command("art-plates")
def assets_art_plates(
    generate: bool = typer.Option(
        False, "--generate", help="Génère aussi les manques (appel payant OpenRouter)"
    ),
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'images générées"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    recrop: bool = typer.Option(
        False, "--recrop", help="Recadre les planches Commons existantes (cache local)"
    ),
    envelope: float = typer.Option(
        8.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
) -> None:
    """AR1 : planches illustrées (chargements, vignettes, fins) dans game/assets/art/."""
    from cent_ans_tools import art_plates, portraits

    written = art_plates.build_commons(force=recrop)
    console.print(f"Commons : {len(written)} planche(s) écrite(s)")
    if not (generate or dry_run):
        return
    jobs = art_plates.generation_jobs()[:limit]
    for family in art_plates.FORMATS:
        family_jobs = [job for job in jobs if art_plates.family_of(job) == family]
        if not family_jobs:
            continue
        _run_art_batch(
            family_jobs,
            portraits.DEFAULT_MODEL,
            envelope,
            dry_run,
            "planche(s)",
            f"AR1 : planches illustrées ({family})",
            lambda image, family=family: art_plates.convert_generated(image, family),
        )


@assets_app.command("codex-art")
def assets_codex_art(
    category: str | None = typer.Option(
        None,
        "--category",
        help="Catégories du Codex séparées par des virgules (défaut : toutes sauf personnages et plantes)",
    ),
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'illustrations"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
) -> None:
    """Génère les miniatures des fiches du Codex sans image d'entité (640×360)."""
    from cent_ans_tools import codex_art, portraits

    categories = (
        tuple(part.strip() for part in category.split(","))
        if category
        else codex_art.CATEGORIES
    )
    _run_art_batch(
        codex_art.plan(categories=categories, limit=limit),
        model or portraits.DEFAULT_MODEL,
        envelope,
        dry_run,
        "illustration(s)",
        "Illustrations du Codex",
        codex_art.convert,
    )


@assets_app.command("materials")
def assets_materials(
    out_dir: Path = typer.Option(  # noqa: B008
        ...,
        "--out",
        help="Dossier des images brutes et tuiles (hors dépôt, ex. scratch)",
    ),
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiants à générer seuls (sonde), répétable"
    ),
    sheet: Path | None = typer.Option(  # noqa: B008
        None, "--sheet", help="Planche de contrôle PNG à écrire"
    ),
    lot: str | None = typer.Option(
        None,
        "--lot",
        help="Préfixe de ligne et plafond du lot (défaut : GA1 ou le bloc budget)",
    ),
    scans: bool = typer.Option(
        False, "--scans", help="Matières scannées (ambientCG) seules, sans appel payant"
    ),
    build: bool = typer.Option(
        False,
        "--build",
        help="Reconstruire les tableaux des figurines (couches sans tuile : conservées)",
    ),
    layers_sheet: Path | None = typer.Option(  # noqa: B008
        None, "--layers-sheet", help="Planche JPEG des couches construites (--build)"
    ),
    config: Path | None = typer.Option(  # noqa: B008
        None,
        "--config",
        help="Fichier de matières (défaut data/art/materials.yaml ; RC5 : water_materials.yaml)",
    ),
    envelope: float | None = typer.Option(
        None,
        "--envelope",
        help="Enveloppe maximale en dollars (abaisse le plafond du bloc budget)",
    ),
    dry_run: bool = typer.Option(
        False,
        "--dry-run",
        help="Affiche les prompts et le coût estimé, sans appel payant",
    ),
) -> None:
    """GA/SR/RC5 : matières tuilables (--config), générées ou scannées, + cartes dérivées.

    Une image brute déjà présente dans --out (ou un scan en cache) est réutilisée sans
    appel. --scans --build : scans SR1 + tableaux, couches générées gardées telles quelles.
    """
    from decimal import Decimal

    import numpy as np
    from PIL import Image

    from cent_ans_tools import material_gen

    path = config or material_gen.MATERIALS_PATH
    document = material_gen.load_materials(path)
    entries = {entry["id"]: entry for entry in document["materials"]}
    ids = only or [
        material_id
        for material_id, entry in entries.items()
        if not scans or material_gen.scan_asset_id(entry)
    ]
    unknown = [material_id for material_id in ids if material_id not in entries]
    if unknown:
        raise typer.BadParameter(f"Matière inconnue dans {path} : {', '.join(unknown)}")
    cap = envelope if envelope is None else Decimal(str(envelope))
    if dry_run:
        report = material_gen.plan(ids, out_dir, materials_path=path)
        for item in report["items"]:
            state = "brute réutilisée" if item["reuse"] else f"{item['cost']} $"
            console.print(f"[bold]{item['id']}[/bold] ({state})\n{item['prompt']}\n")
        limit = report["cap"] if cap is None else min(cap, report["cap"] or cap)
        console.print(
            f"Modèle {report['model']} : {len(report['items'])} matière(s), "
            f"coût estimé {report['total']} $"
            + (f" (plafond {limit} $)" if limit is not None else "")
        )
        if limit is not None and report["total"] > limit:
            console.print("[red]Estimation au-dessus du plafond.[/red]")
            raise typer.Exit(1)
        return
    tiles = {}
    for material_id in ids:
        albedo = material_gen.generate(
            material_id, out_dir, lot=lot, materials_path=path, envelope=cap
        )
        tiles[material_id] = np.asarray(Image.open(albedo).convert("RGB"))
        console.print(f"[green]OK[/green] : {material_id} -> {albedo}")
    if sheet is not None:
        contact = material_gen.contact_sheet(tiles, sheet, params=entries)
        console.print(f"Planche : {contact} ({contact.stat().st_size // 1024} Ko)")
    if build:
        for kind, path in material_gen.build_fine_arrays(out_dir).items():
            console.print(f"Tableau {kind} : {path}")
        if layers_sheet is not None:
            path = material_gen.layers_sheet(layers_sheet)
            console.print(f"Planche : {path} ({path.stat().st_size // 1024} Ko)")
    console.print(f"Cumul de la section : {budget.total()} $")


@assets_app.command("ui-ornaments")
def assets_ui_ornaments(
    dry_run: bool = typer.Option(False, "--dry-run", help="Estime sans appel payant"),
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiants à traiter seuls, répétable"
    ),
    sheet: Path | None = typer.Option(  # noqa: B008
        None, "--sheet", help="Planche avant/après PNG à écrire"
    ),
    install: bool = typer.Option(
        False, "--install", help="Installer les variantes `selected` dans nb/"
    ),
) -> None:
    """NB : kit d'interface redessiné (data/art/ui_ornaments.yaml)."""
    import json

    from cent_ans_tools import ui_ornaments as orn

    config = orn.load_ornaments()
    kit = json.loads((orn.KIT_DIR / "kit.json").read_text(encoding="utf-8"))
    if install:
        selected = {
            entry["id"]: entry["selected"]
            for entry in config["pieces"]
            if "selected" in entry and (not only or entry["id"] in only)
        }
        written = orn.install(selected, kit)
        for path in written:
            console.print(f"[green]OK[/green] : {path}")
        if sheet is not None:
            before = [orn.KIT_DIR / f"{path.stem}.png" for path in written]
            console.print(f"Planche : {orn.contact_sheet(before, written, sheet)}")
        return
    if only:
        config = {
            **config,
            "pieces": [e for e in config["pieces"] if e["id"] in only],
            "decor": [e for e in config["decor"] if e["id"] in only],
        }
    anchor = orn.ANCHOR_PATH.read_bytes() if orn.ANCHOR_PATH.exists() else b""
    requests = orn.build_requests(kit, anchor, config)
    orn.generate(requests, dry_run=dry_run, model=config["model"])
    console.print(f"Cumul : {budget.total()} $")


if __name__ == "__main__":
    app()


ground_app = typer.Typer(
    help="HB2 : matières de sol de la carte de campagne (fal.ai, ADR 0143).",
    no_args_is_help=True,
)
assets_app.add_typer(ground_app, name="ground-materials")


@ground_app.command("generate")
def ground_generate(
    only: list[str] = typer.Option(  # noqa: B008
        None, "--only", help="Identifiants à générer seuls, répétable"
    ),
    attempt: int | None = typer.Option(
        None, "--attempt", help="Reprise (graine + 1000 par reprise)"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche les appels et leur coût, sans appel payant"
    ),
) -> None:
    """Génère les images brutes manquantes (hors dépôt, jamais payées deux fois)."""
    from cent_ans_tools import ground_materials

    catalog = ground_materials.load_catalog()
    report = ground_materials.generate(
        catalog, only=only or None, attempt=attempt, dry_run=dry_run
    )
    verb = "prévus" if dry_run else "faits"
    console.print(
        f"{len(report['planned'])} appel(s) {verb}, {report['cost']:.3f} $ "
        f"({catalog['model']})"
    )
    if dry_run:
        for name in report["planned"]:
            console.print(f"  {name}")


@ground_app.command("seamless")
def ground_seamless(
    only: list[str] = typer.Option(  # noqa: B008
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
