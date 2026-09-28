# TW2 lot SB — lisibilité et rythme de la destruction en siège

Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § SB. Branche `feat/tw2-sb`. ADR 0100.

## État
- [x] Sonde `core/crates/sim-battle/tests/sb_siege_pace.rs` (tableau par niveau, tests de cibles niveau 3 / niveau 5).
- [x] Données `data/rules/siege_works.json` retouchées (mur 100+200/niv., porte 40+70/niv.).
- [ ] Cœur : `under_attack` par pièce + PV bélier/tours exposés dans `get_siege`.
- [ ] Vérifier tests de siège existants (sg3 invariant mur ≥ 3× porte à relâcher, sg4_balance, br3, b6).
- [ ] Godot : barres de vie flottantes + étiquette « Porte : 320/540 » ; test `game/tests/sb_siege_bars_test.gd`.
- [ ] ADR 0100 avec tableau avant/après.

## Prochaine étape
Cœur `under_attack`.
