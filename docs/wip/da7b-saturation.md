# Lot DA7b — Saturation en automne et en bocage

Branche `feat/da7b-saturation` (worktree `.claude/worktrees/agent-ae97dd783f8ba4906`).
Bible : `docs/design/2026-09-25-bible-da.md` § 3.3 (saturation HSV moyenne ≤ 35 % en plein jour).
Suite de DA6 (ADR 0067 § « DA7b », `docs/wip/da6-vegetation-bataille.md`). Rendu et données
seulement (aucun Rust ; dylib copiée de main, pas de build).

## État : terminé, à fusionner par l'orchestrateur
- [x] Outil de mesure `tools/cent_ans_tools/scene_saturation.py` (+ tests) : reprise du script
  jetable de DA6 (même zone), teinte moyenne en plus ; référence DA6 reproduite à 0,1 % près.
- [x] Captures avant (main) et après : `docs/img/da7b/avant_*`, `apres_*` (13 vues).
- [x] `shadow_saturation` (LUT d'étalonnage) et bloc `battle_seasons` (étalonnage de saison en
  bataille, `decor_saturation` par saison sorti du code) dans `data/fx/atmosphere.json` + schéma.
- [x] ADR 0067 § DA7b (tableau complet).
- [x] pytest 768 OK, smoke Godot exit 0.

## Commandes
- `uv run --project tools python -m cent_ans_tools.scene_saturation measure docs/img/da7b/*.jpg`
- `uv run --project tools python -m cent_ans_tools.scene_saturation capture docs/img/da7b --prefix=apres_`
  (`--views=automne,bocage_haies`, `--extra=--no-da6`) ; ~70 s pour les 13 vues sur M4 Pro.

## Résultat (avant → après)
automne 44,7 → 32,7 % ; bocage haies 39,1 → 33,7 % ; bocage haute 36,1 → 31,3 % ; automne haute
43,4 → 28,1 % ; automne en bocage 49,7 → 32,4 % ; bois 37,0 → 32,3 % ; vue haute 33,7 → 29,4 % ;
hiver 15,2 → 15,3 % (non touché). Teinte inchangée à ± 2°. Tableau : ADR 0067.

## Limites / suites
- `--terrain=bocage --closeup` (vue DA6) cadre un champ ouvert sur main : vues `bocage_haies`
  et `bocage_haute` ajoutées. `--closeup` en général ne cadre plus de mêlée à 45 s (hors lot).
- `--standard-shot=foot` : cadrage variable d'un lancement à l'autre (cavaliers ou mur de pont).
- Marge du bocage au printemps faible (33,7 %).
- La campagne garde l'étalonnage `seasons` (non mesurée ici).
