# SZ2 — fonds de vallée E0-E4 (défaut S2 de ZG7c)

Chantier SZ (`docs/wip/sz-suites-zoom.md`). Branche `feat/sz2-valley-floors` (worktree d'agent
`agent-acc401f3ef1c1478d`, depuis `main` 7e1ac032). Agent « données » : pas de compilation Rust.

## Mise en place (hors git)
- `data/map/pyramid` du worktree → lien vers `/Users/jean_hubert/dev/game_project/data/map/pyramid.sz2`
  (dossier de préparation ; le cache partagé `data/map/pyramid` n'est jamais réécrit).
- `tools/geo/raw` du worktree : dossier réel ; liens vers les bruts en lecture seule du dépôt principal
  (copernicus, copernicus30, worldcover, etopo2022, …) ; **clones APFS** (`cp -c -R`) de `detail/`,
  `hydro/`, `pyramid_work/` (ces trois dossiers reçoivent des marqueurs / caches réécrits par la
  cuisson : ne pas toucher ceux du dépôt principal).

## Diagnostic (mesures avant, `scratchpad/probe.py`)
| Lieu | réel (m) | GLO-90 min | base σ 5 km | E0 | E2 | E4 |
|---|---:|---:|---:|---:|---:|---:|
| Paris | 27 | 31,6 | 59 | 9,7 | 0,5 | 0,5 |
| Vernon | 12 | 11,0 | 103 | 0,5 | 0,5 | 0,5 |
| Rouen | 3 | 4,0 | 90 | 0,5 | 0,5 | 0,5 |
| Orléans | 92 | 90,4 | 110 | 74,8 | 71,2 | 67,3 |
| Amboise | 55 | 51,4 | 93 | 18,2 | 15,6 | 15,1 |
| Tours | 46 | 42,5 | 79 | 12,7 | 11,8 | 7,1 |

Cause : le rehaussement de rendu (ADR 0019, masque flou σ 5 km, gain 0,8, ±120 m) **creuse** les
vallées encaissées de 0,8 × (base − fond) : la base σ 5 km contient les plateaux, d'où 30 à 90 m
de creusement, puis le plancher de côte 0,5 m. E5-E7 (même formule, autre base) creusent
autrement : d'où les écarts d'étage à étage (Loire d'Orléans sous ses berges E7). ZG8 exagère déjà
à l'exécution le relief *au-dessus du fond de vallée* : le creusement cuit fait double emploi.

## Correctif (outils)
- `relief_shade.valley_floor(h) = max(0,5 ; 0,85 h ; h − 2 m)` : plancher **monotone** de la terre
  rehaussée, commun à E0 (`boost_relief`), E1-E4 (`pyramid.boost_with_base`, donc aussi
  `surface.tier2_heights`) et E5-E7 (`detail_dem.apply_boost`, remplace `low_land_floor` de ZG7a,
  identique sous 5,9 m). Les collines restent rehaussées ; les fonds de vallée restent à ≤ 2 m de
  leur altitude réelle quel que soit l'étage → continuité E0/E1…E7.
- Versions : `pyramid.BAKE_VERSION = 2` (E1-E4), `detail_dem.BAKE_VERSION = 5` (E5-E7, clé des
  recalages `hydro-fine`). Nouveau module `geo/bake_stamp.py` : le cache porte
  `pyramid/bake.json` (version, début, fini) ; le manifeste versionné porte `bake_versions`.
  Un palier périmé est signalé par `relief-all --check` et recuit par `geo pyramid` /
  `detail-dem` / `relief-all` sans `--force` ; une cuisson interrompue reprend (tuiles plus
  anciennes que son début seulement).

## État
- [x] Correctif outils + tests (`tests/test_bake_stamp.py`, tests adaptés) — e13efe90
- [x] E0 recuit (`geo relief-shade`, 176 tuiles sur 256 changent) — 22cb2e06.
  E0 après : Paris 29,6 ; Vernon 9,4 ; Rouen 3,4 ; Orléans 88,4 ; Amboise 49,4 ; Tours 40,5.
- [x] E1-E4 recuits dans `pyramid.sz2` (9 833 tuiles, 2,44 Go, 12 min). E4 : Paris 23,3 ; Vernon 6,8 ; Orléans 84,3 ; Blois 60 ; Amboise 48 ; Tours 37,4 (avant 0,5 ; 0,5 ; 67 ; 31 ; 15 ; 7)
- [ ] `detail-dem --force` EN COURS (journal `scratchpad/logs/detail.log`), puis `hydro-fine`, `anchors-fine`
- [ ] Vérifications (Orléans, Amboise, Seine, `detail-check`, `relief-all --check`)
- [ ] Captures, docs, commande de bascule

## Commande en cours (reprise : relancer telle quelle, elle saute les tuiles déjà recuites)
```sh
cd <worktree> && uv run --project tools cent-ans geo pyramid --levels 1,2,3,4 --workers 12
```
Journal : `scratchpad/logs/pyramid.log`. Ensuite, dans l'ordre :
`geo detail-dem --force` (ou sans : l'estampille périmée suffit), `geo hydro-fine`,
`geo anchors-fine`, puis `geo relief-all --check`.

## Prochaine étape
Attendre la fin de E1-E4, puis la suite ci-dessus.
