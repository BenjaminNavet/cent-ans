# Lot C3 : pipeline géo des colonies

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 3.3, 3.4, 4.1, 4.4, 5. Doc : `docs/geo.md`.

## État : terminé

- [x] `settlements` : projection, contrôle province, graphe, `settlements_px.json`, aperçu
- [x] `roads` : Itiner-e (Zenodo 17122148, gpkg sans compte) + routes calculées hors limes
- [x] `hamlets` : GeoNames `cities500.zip`, 2 999 hameaux
- [x] `relief` : 256 tuiles 512², 50,5 Mo
- [x] CLI (`geo settlements|roads|hamlets|relief`, `geo build` les enchaîne), tests
  `tools/tests/test_settlement_graph.py`, `docs/geo.md`, précisions de la spec § 5

## Prochaine étape

Données complètes (568 colonies) intégrées. Si des fichiers de colonies changent : `uv run --project tools cent-ans geo roads`
(routes + graphe) puis `cent-ans geo hamlets`, commiter `data/map/{roads.geojson,
settlement_graph.json,settlements_px.json,hamlets.json}` et `docs/img/settlements-preview.png`.

## Notes

- Cache brut : `tools/geo/raw/` (dans le worktree, lien symbolique vers celui du dépôt principal).
- `geo build` régénère aussi `provinces.geojson` avec une petite différence (non-déterminisme
  préexistant, hors lot) : non commitée.
