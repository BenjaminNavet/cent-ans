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

## État
- [ ] Correctif outils (plancher de vallée commun E0-E7) + tests
- [ ] E0 recuit (`geo relief-shade`), commité
- [ ] E1-E4 recuits dans `pyramid.sz2`
- [ ] `detail-dem --force`, `hydro-fine`, `anchors-fine`
- [ ] Vérifications (Orléans, Amboise, Seine, `detail-check`, `relief-all --check`)
- [ ] Captures, docs, commande de bascule

## Prochaine étape
Écrire le correctif dans `relief_shade.py` / `pyramid.py` / `detail_dem.py`.
