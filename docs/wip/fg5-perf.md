# FG5 — Performance et bascule des figurines fines par défaut

Branche : `feat/fg5-perf` (worktree `.claude/worktrees/fg5-perf`). Plan : `docs/wip/fg-figurines-fines.md`.
Précédents : `fg1-corps.md`, `fg3-matieres.md`, ADR 0088.

## État
- [ ] Banc de référence (défaut / fine, 4 passes alternées)
- [ ] Leviers de perf : relais LOD0 plus tôt, `fine_distance` 80 → 40 m, tuiles au LOD0 seulement,
  LOD1 allégé (décimation + recuisson)
- [ ] Bascule : rendu fin par défaut, drapeau pour l'ancien rendu skinné, `--no-fg3` gardé
- [ ] Vérifs : smoke, tests figurines, captures `docs/img/fg/fg5_*.png` (campagne, menu, cadavres,
  imposteurs, étendards, DA1, EP12)
- [ ] ADR 0089 + bible § 6 + journal FG

## Prochaine étape
Banc de référence.

## Commandes
```
cp /Users/jean_hubert/dev/game_project/game/bin/*.dylib game/bin/
godot --headless --path game --import
godot --path game --resolution 1600x900 res://scenes/battle/battle.tscn -- --units=50 --benchmark --bench-at=90 [--fine-figures]
```
Piège : `--path game` (pas la racine du worktree : « [unnamed project] » qui attend sans fin).
