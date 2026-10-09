# QW-C — budget réconcilié et crédits IA

État : terminé (09/10).

- `docs/budget.md` : synthèse en tête (par fournisseur, par enveloppe, total général 116,78 $), plafond v1
  de 50 $ NON modifié, dépassement signalé pour décision du joueur ; décimales harmonisées, cumuls recalculés,
  date `2026-10-10` corrigée en `2026-10-09` ; rapprochement fal (journal 43,14 $ vs registre 42,43 $, écart 0,71 $,
  cause non établie).
- `tools/cent_ans_tools/budget.py` : `grand_total()`, `service_totals()`, `render_summary()`, totaux calculés
  sur « Coût réel » (colonne Cumul ignorée) ; commande `cent-ans budget summary` ; tests dans `test_budget.py`.
  Contrainte : la synthèse de `budget.md` ne doit contenir ni ligne `##` ni tableau (le parseur les lirait comme sessions).
- `CREDITS.md` : section « Contenus générés par IA » (format de titres inchangé, lu par `credits_screen.gd`).
- `LICENSE-ASSETS.md` : même licence pour les assets IA, tableau des conditions de modèles (« à vérifier » quand incertain).

Points ouverts : `DN_FAL_CAP_USD` défaut 43,5 $ dans `tools/experiments/dn_batch.py` (hors lot) ; la synthèse de
budget.md est écrite à la main (à régénérer avec `budget summary`).
