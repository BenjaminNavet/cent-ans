"""Geo pipeline: builds the campaign map terrain (``data/map/``) from open data.

Modules:
    download: fetch and cache Natural Earth and ETOPO 2022 raw files.
    project: EPSG:3035 grid definition and pixel <-> projected helpers.
    terrain: heightmap and land mask rasters.
    vectors: rivers and coastline in map pixel coordinates.
    build: orchestration (``cent-ans geo build``).
"""
