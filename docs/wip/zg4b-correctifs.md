# ZG4b — correctifs de la vue rapprochée relevés en recette (Q3)

Worktree d'agent (depuis `main` 56baaf9a). Liens symboliques non versionnés `data/map/pyramid`,
`tools/geo/raw`. Dylib : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`.
Rendu seulement.

## Problèmes
1. Sol beige nu au plus près au-dessus des villes emblématiques (Londres 0,3-3 u.).
2. Ruban rouge et gris géant traversant Londres vers 20 u. (`en3-011`).
3. Ponts-portes à l'échelle exagérée ; pic de ~35 ms au basculement des ponts en mode fin.

## État
- [x] reproduction : ruban reproduit (`--focus=2018,1486.6,20 --dump-near`)
- [ ] identification du nœud du ruban
- [ ] plancher de caméra déclaratif au-dessus des zones emblématiques
- [ ] ponts-portes, étalement du basculement
- [ ] tests, captures `docs/img/zg4b/`, doc

## Prochaine étape
Identifier le nœud du ruban (TradeRouteLayer ? Crossings ? FineGeo/Roads ?).
