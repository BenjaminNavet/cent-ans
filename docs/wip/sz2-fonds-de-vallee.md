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

## État : lot terminé (reste la bascule par l'orchestrateur, puis la fusion)
- [x] Correctif outils + tests (`tests/test_bake_stamp.py`, tests adaptés) ; pytest 697 OK, ruff OK
- [x] E0 recuit (`geo relief-shade`, 176 tuiles sur 256 changent), commité dans la branche
- [x] E1-E4 recuits dans `pyramid.sz2` (9 833 tuiles, 2,44 Go, 12 min sur 12 processus)
- [x] `detail-dem --force` (7 min), `hydro-fine` (9,5 min), `anchors-fine` (6 min) ; manifestes
  `relief_pyramid.json` (`bake_versions` 2/2/5), `rivers_fine.json`, `fine_anchors.json` commités
- [x] `relief-all --check` : complet, 2,69 Go, aucun palier périmé
- [x] Captures `docs/img/sz2/` (Orléans, Val de Loire, Rouen, Paris ; paliers vallée et site ;
  « avant » = `docs/img/zg7c/`) ; smoke Godot OK

## Mesures (avant = cache partagé actuel, après = `pyramid.sz2`)
Fleuve fin le plus large à ≤ 1,5 km du lieu : niveau d'eau médian, berges (relief le plus fin à
+15/30/60 m des rives, p10-p90), écart berge basse − eau (`scratchpad/water_check.py`).

| Lieu | eau avant | berges avant | eau après | berges après | écart après |
|---|---:|---:|---:|---:|---:|
| Orléans (E7) | 76,2 | 75,5-91,7 | 86,6 | 86,4-94,9 | −0,1 m |
| Blois (E4) | 28,5 | 30,5-47,7 | 60,0 | 63,4-71,0 | 4,4 m |
| Amboise (E4) | 16,4 | 15,6-32,4 | 46,7 | 47,5-51,9 | 0,4 m |
| Tours (E4) | 11,5 | 11,5-28,8 | 40,0 | 39,9-49,2 | 0,2 m |
| Paris (E7) | 4,9 | 15,4-21,2 | 24,8 | 30,2-33,5 (quais) | 6,8 m |
| Poissy (E4) | 0,5 | 0,5 | 14,9 | 13,8-18,3 | 1,1 m |
| Vernon (E4) | 0,5 | 0,5 | 6,7 | 6,8-12,9 | 0,1 m |
| Rouen (E7) | 0,9 | 4,4-5,0 | 2,9 | 4,4-9,2 | 1,6 m |
| Londres (E7) | 1,9 | 2,9-5,0 | 2,0 | 2,9-5,2 | 1,9 m |

`geo detail-check` (écart E5/E4 sur terre) : toutes les zones s'améliorent (médianes 0,5-18 m →
0,5-5,4 m ; Carcassonne p95 31 → 19 m, Reims 22 → 7,8 m, Paris 13 → 9,2 m) ; le seuil d'alerte
de 5 m (p95) reste dépassé par 31 zones sur 34, comme avant (différence de source et de
résolution MNT 1-5 m / GLO-30 corrigé à 22 m : falaises, bâti, remblais).

## Limites
- Les valeurs absolues restent 2-6 m sous le réel dans les vallées (source GLO-30 corrigée +
  2 m de creusement toléré) : Amboise 46,7 m (réel ≈ 52), Orléans 86,6 (≈ 90), Seine à Paris 24,8
  (≈ 26-27). L'objectif « au niveau des berges » est tenu ; « Loire à 55 m à Amboise » ne l'est
  qu'à ≈ 6 m près.
- « Marches sombres » du Val de Loire : atténuées (fond non creusé) mais les bourrelets E4 (levées,
  terrasses de la source) restent visibles au palier site.
- `horizon.py` lit aussi les tuiles E0 (`height/`) : silhouette d'horizon non recuite (effet
  négligeable, vallées invisibles à l'horizon).
- Aucune autre donnée versionnée ne dépend du relief de rendu E0 (`splat`, `navgrid`,
  `river_render`, `landcover` lisent `heightmap.png`, inchangé).

## Bascule (orchestrateur)
Quand plus aucun agent ne lit ni n'écrit la pyramide partagée, **en même temps que la fusion de la
branche** (E0 et manifestes commités doivent aller avec le cache) :
```sh
cd /Users/jean_hubert/dev/game_project/data/map \
  && mv pyramid pyramid.pre-sz2 && mv pyramid.sz2 pyramid \
  && uv run --project /Users/jean_hubert/dev/game_project/tools cent-ans geo relief-all --check \
  && rm -rf pyramid.pre-sz2
```
Facultatif (caches de travail cohérents avec le nouveau cache, pour un futur `relief-all`) :
remplacer dans `tools/geo/raw/` du dépôt principal `detail/`, `hydro/`, `pyramid_work/` par les
clones du worktree (`<worktree>/tools/geo/raw/{detail,hydro,pyramid_work}`, marqueurs
`done_E*` v5, recalages v5, `e0.npy` du nouvel E0). Sans cela, rien ne casse : les marqueurs v4
feraient seulement recuire E5-E7 si l'on relançait `detail-dem`.
Espace : `pyramid.sz2` 2,5 Go ; disque libre ≈ 39 Go.

## Prochaine étape
Bascule + fusion par l'orchestrateur.
