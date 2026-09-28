# TW2 lot SB — lisibilité et rythme de la destruction en siège

Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § SB. Branche `feat/tw2-sb`. ADR 0100.

## État
- [x] Sonde `core/crates/sim-battle/tests/sb_siege_pace.rs` (tableau par niveau, tests de cibles niveau 3 / niveau 5).
- [x] Données `data/rules/siege_works.json` retouchées (mur 100+200/niv., porte 40+70/niv.).
- [x] Cœur : `WallPiece::under_attack()` (+ `attacked_for`), `BattleSim::siege_engines()` ; `get_siege` expose `pieces[i].under_attack` et `engines`.
- [x] Tests de siège : invariant sg3 relâché (mur ≥ 1,5× porte), test du feu de porte raccourci (20 s), sg1 bélier/engin passés au niveau 5. Taux de prise sg3 inchangé ; banc br3 bascule 3/10 → 10/10 (documenté ADR 0100, levier T4).
- [x] Godot : `game/scripts/battle/siege_health_bars.gd` + `game/tests/sb_siege_bars_test.gd` (OK) ; smoke OK.
- [x] ADR 0100 avec tableaux avant/après.

## Prochaine étape
Lot terminé ; reste la vérification visuelle (session principale, pas de capture en sous-agent).
