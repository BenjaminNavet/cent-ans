# HC5 — nouveaux massifs, landes et zones humides historiques (vers 1340)

Worktree `../gp-hc5`, branche `feat/hc5`. ADR 0161 (complément du 02/10). Pyramide et
`tools/geo/raw` en liens symboliques vers le checkout principal (lecture seule).

## Mandat
Ajouter ≈ 60-90 massifs / landes et ≈ 30-45 zones humides attestés vers 1340 sur toute la carte
(`data/map/historical_forests.json`, `data/map/wetlands.json`), sourcés, puis recuire
`landcover` → `navgrid` → `colormap` → `horizon` et mesurer l'écart de règles (grille de
déplacement, couvert). Objectif : forêt globale ≤ ≈ 45 % des terres, aucun lieu isolé, aucune
liaison renchérie de plus de 30 % sans correction.

## État
- [ ] Squelette : note, schémas étendus à toute la carte (lon −11..61, lat 28..66).
- [ ] Mesure « avant » (script hors dépôt, résultats ci-dessous).
- [ ] Données par région (France, îles Britanniques, Ibérie, Italie, Empire, Est, Balkans,
      Maghreb / Orient).
- [ ] Recuisson et mesure « après ».
- [ ] Tests (pytest, Rust `real_data`, Godot import + smoke), `docs/geo.md`.

## Prochaine étape
Étendre les schémas, mesurer l'état de référence, puis saisir les entrées par région.

## Points ouverts
(à compléter)
