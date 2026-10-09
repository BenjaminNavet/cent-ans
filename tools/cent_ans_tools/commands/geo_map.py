"""Groupe ``geo`` (1/2) : build, provinces, routes, hydrographie, villes."""

from __future__ import annotations

from pathlib import Path

import typer

from cent_ans_tools.commands._common import console
from cent_ans_tools.commands._geo_common import (
    _print_sizes,
    _report_navgrid,
    _report_provinces,
)

geo_app = typer.Typer(
    help="Carte de campagne : terrain depuis ETOPO / Natural Earth, provinces.",
    no_args_is_help=True,
)


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


@geo_app.command("agri-regions")
def geo_agri_regions() -> None:
    """Paysages agricoles régionaux (lot ME8) : agri_regions.png depuis agri_landscapes.json."""
    from cent_ans_tools.geo import agri_regions

    path = agri_regions.build()
    console.print(f"{path.name} : {path.stat().st_size / 1e3:.0f} ko")


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


@geo_app.command("freshwater-sites")
def geo_freshwater_sites() -> None:
    """Génère freshwater_px.json (zones humides, salins, cascades, gués, torrents ; lot DN-ME4)."""
    from cent_ans_tools.geo import freshwater

    result = freshwater.build()
    console.print(
        f"{len(result['wetlands'])} zones humides, {len(result['salt_pans'])} salins, "
        f"{len(result['waterfalls'])} cascades, {len(result['fords'])} gués, "
        f"{len(result['torrents'])} tronçons de torrent"
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
    city: list[str] = typer.Option(
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
