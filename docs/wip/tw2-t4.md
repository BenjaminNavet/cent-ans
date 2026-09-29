# TW2 lot T4 — points de capture en siège (bataille) et rééquilibrage des assauts

Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T4. Branche `feat/tw2-t4` (depuis
`feat/tw2-sb`). ADR **0108** (0104 pris ailleurs ; SB renuméroté 0107).

## État
- [x] Règles `data/rules/siege_capture.json` + schéma + pytest ; cœur `sim-battle/src/capture.rs`,
      `sim/capture.rs` (points place/porte, victoire, porte ouverte + tours muettes, dernier carré,
      alerte `square_threatened`).
- [x] IA : garnison repliée d'un bloc sur la place dès la première brèche ; assaut groupé (60 %).
- [x] Pont `get_siege().points` ; Godot `siege_capture_points.gd` (drapeaux, cercles, barres),
      alerte « La place est menacée » ; `game/tests/t4_capture_points_test.gd` OK.
- [x] Tests Rust `tests/t4_capture.rs` (6).
- [x] Mesures : br3 7/2/3 (10 graines), 14/4/8 (20 graines) ; sg3 inchangé sauf Rouen 7 → 8/10. ADR 0108.
- [x] Fusion de main (29/09, orchestrateur), correctif cavalier (9145808e), test de brèche adapté ; tests complets.

## Points ouverts
- Paris reste à 2/10 sur br3 (rues étroites, chaleur des incendies) : voir ADR 0108.
- Icône à l'encre DA5 pour `battle_alert_square_threatened` (glyphe vectoriel en attendant).
