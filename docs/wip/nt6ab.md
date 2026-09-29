# NT6ab petites suites

Branche `feat/nt6ab-leftovers`.

## Etat
- [x] 1 discours du general adverse (battle_scene `_start_enemy_speech`, `BattleSpeech.skipped`)
- [x] 2 indicateur vise beliers/beffrois (`BattleSiege._update_target_marks`)
- [x] 3 surprime mercenaires (pont `mercenary_premium*`, ligne `BudgetTable`)
- [~] 4 en-tete de province compacte (`ProvincePanel._compact_header`), avis longs (`TOAST_MAX_LINES`)
- [ ] test `game/tests/nt6ab_test.gd` a lancer, tests UI existants

## Prochaine etape
Build dylib (`core/build.sh`), import Godot, lancer nt6ab_test, smoke, po_ui, q6_*.
