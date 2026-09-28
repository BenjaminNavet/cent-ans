# TW2 lot T4 — points de capture en siège (bataille) et rééquilibrage des assauts

Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T4. Branche `feat/tw2-t4` (depuis
`feat/tw2-sb`). ADR 0104.

Objectif chiffré : `br3_assault_probe` revenu à **4-7/10** prises (SB l'a fait passer à 10/10),
sans rallonger la destruction des murs ; `sg3_assault_probe` proche de l'actuel.

## État
- [x] Squelette : `data/rules/siege_capture.json` + schéma + pytest ; `core/crates/sim-battle/src/capture.rs`
      (règles, `CapturePoint`, `advance`).
- [ ] Mesure « avant » (br3, sg3) sur l'état SB.
- [ ] Cœur : points dans `SiegeWorks`, pas de capture (`sim/capture.rs`), victoire par la place, porte
      prise (ouverture + tours muettes), dernier carré (moral), alerte `square_threatened`.
- [ ] IA : repli de la garnison sur la place dès la première brèche ; l'assaillant converge.
- [ ] Pont : `get_siege().points`.
- [ ] Godot : drapeaux au sol + barre de capture, alerte « La place est menacée », test headless.
- [ ] Réglage br3 4-7/10, sg3 inchangé ; ADR 0104 avec chiffres avant/après.
- [ ] Fusion `integration/tw2`, commit final.

## Prochaine étape
Mesure avant (probe br3 en cours), puis branchement du pas de capture dans `resolve_siege_works`.
