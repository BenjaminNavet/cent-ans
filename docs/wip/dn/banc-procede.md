# DN banc du procédé (08/10)

État : outil `tools/experiments/dn_batch.py` + `tools/gpu_lock.sh` faits et vérifiés de bout en bout sur
moulin et chariot (voir `docs/pipeline-assets-3d.md` § 7). Sorties : `~/dev/cent-ans-raw/dn/`.

- fal : crédit OK (3 appels, 0,06 $, consignés dans `docs/budget.md`).
- HF TRELLIS : quota ZeroGPU épuisé le 08/10 (échec journalisé, étape optionnelle).
- Contrôle D5 de la bible : trop strict pour le bois (p95 de S ≈ 0,85); `--charter warn` pour la production
  tant que le seuil n'est pas révisé.
- Qwen absent du banc (trop lent).

Prochaine étape : production campagne puis bataille avec `data/art/dn_catalog_*.json`.
