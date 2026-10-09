# DN-FIX3 : canons primitifs et figures montées

État : FAIT (2026-10-09). Dépense 0,65 $ (plafond lot 1 $).

- Canons : `siege_cannon_early_1340` (graine 1338, pot-de-fer en bouteille sur madrier) et
  `siege_bombard_hooped_1380` (graine 1337, tube à douelles cerclées, âme visible, caisson de madriers
  calé de coins et pieux) régénérés (2 passes de prompt), ingérés par `dn-ingest`, glb copiés dans le
  checkout principal.
- Figures montées : nouveau gabarit cavalier (`MOUNTED_PREFIX/SUFFIX`, entrée `"mounted": true`) et
  prompt de vue dos dédié (`VIEW_PROMPTS_MOUNTED`) dans `tools/experiments/dn_batch.py`. Refaits, graine
  1338 : `army_lord_mounted`, `cavalry_1_druzhina`, `cavalry_4_routier`, `cavalry_5_jinete`,
  `fig_army_lord_mounted_islamic`, `fig_army_lord_mounted_rus`. Les trois autres (`byzantine`, `steppe`,
  `messenger_rider`) montraient déjà un seul cavalier sur un cheval entier : conservés.
- Anciens artefacts : `~/dev/cent-ans-raw/dn/<id>/rejected_v1/` (canons : aussi `rejected_v1b/`).
- Glb figures : `~/dev/cent-ans-raw/dn/<id>/3d/fal__s1338.glb` (hors ingest ; à passer par ga3_figures.py).

## Pour une session locale
Aucun id (aucun échec ni refus fal).
