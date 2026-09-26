# Lot DA7b — Saturation en automne et en bocage

Branche `feat/da7b-saturation` (worktree `.claude/worktrees/agent-ae97dd783f8ba4906`).
Bible : `docs/design/2026-09-25-bible-da.md` § 3.3 (saturation HSV moyenne ≤ 35 % en plein jour).
Suite de DA6 (ADR 0067, `docs/wip/da6-vegetation-bataille.md`). Rendu et données seulement.

## Outil de mesure
`tools/cent_ans_tools/scene_saturation.py` (reprise du script jetable de DA6, même zone mesurée) :
- `uv run --project tools python -m cent_ans_tools.scene_saturation measure docs/img/da6/*.jpg`
- `... capture docs/img/da7b --prefix=apres_` (vues DA6 : closeup, foot, haute, ligne, hiver,
  automne, bocage, bois, bois_hiver ; 1600 × 900).
Référence reproduite sur `docs/img/da6/apres_*` : automne 44,1 %, bocage 41,2 %, bois 36,0 %,
haute 33,4 %, closeup 29,0 %, ligne 26,5 %, foot 24,5 %, bois_hiver 20,4 %, hiver 14,7 %.

## État
- [x] Outil de mesure + tests, référence reproduite.
- [ ] Captures « avant » sur main.
- [ ] Réglages par saison (données) : automne, bocage.
- [ ] Captures après, tableau, ADR 0067 § DA7b.

## Prochaine étape
Captures avant (`capture docs/img/da7b --prefix=avant_`), puis réglages.
