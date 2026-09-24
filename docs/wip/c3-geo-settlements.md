# Lot C3 : pipeline géo des colonies

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 3.3, 3.4, 4.1, 4.4, 5.

## État

- [x] Squelette des modules `settlements`, `roads`, `hamlets`, `relief` (`tools/cent_ans_tools/geo/`)
- [x] `settlements` : projection, contrôle province, graphe, `settlements_px.json`, aperçu
- [x] `roads` : Itiner-e (Zenodo 17122148, gpkg, téléchargeable sans compte) ou repli calculé
- [ ] `hamlets` : GeoNames `cities500.zip`
- [ ] `relief` : tuiles 8192² 16 × 16
- [ ] CLI + `geo build`, tests, `docs/geo.md`

## Prochaine étape

Hameaux, relief, tests, docs.

## Notes

- Cache brut : `tools/geo/raw/` (dans le worktree, lien symbolique vers celui du dépôt principal).
