# Lot MEM : memoire du client Bevy

## Etat
- Sonde `memory` ajoutee au dump d etat (RSS, pic, images CPU/GPU, top 10) et au rapport des bancs (`memory.peak_rss_mib`).
- Mesure de base (release, Ultra, hors fenetre) : voir ci-dessous.

## Prochaine etape
- Identifier les gros postes (footprint, pas RSS : le compresseur macOS fausse RSS), corriger, ADR 0748, garde perf_guard.

## Notes
- `malloc_zone_pressure_relief` essaye puis abandonne : RSS baisse (pages compressees) mais footprint inchange.
