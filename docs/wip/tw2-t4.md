# TW2 lot T4 — points de capture en siège (bataille) et rééquilibrage des assauts

Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T4. Branche `feat/tw2-t4` (depuis
`feat/tw2-sb`). ADR 0104.

Objectif chiffré : `br3_assault_probe` revenu à **4-7/10** prises (SB l'a fait passer à 10/10),
sans rallonger la destruction des murs ; `sg3_assault_probe` proche de l'actuel.

## État
- [x] Squelette : `data/rules/siege_capture.json` + schéma + pytest ; `core/crates/sim-battle/src/capture.rs`
      (règles, `CapturePoint`, `advance`).
- [x] Mesure « avant » br3 sur l'état SB : 10/10, 10/10, 10/10 (201 / 242 / 328 s). sg3 à mesurer.
- [x] Cœur : points dans `SiegeWorks`, pas de capture (`sim/capture.rs`), victoire par la place, porte
      prise (ouverture + tours muettes), dernier carré (moral), alerte `square_threatened`.
- [~] IA : repli de la garnison sur la place dès la première brèche ; l'assaillant converge.
- [x] Pont : `get_siege().points`.
- [ ] Godot : drapeaux au sol + barre de capture, alerte « La place est menacée », test headless.
- [ ] Réglage br3 4-7/10, sg3 inchangé ; ADR 0104 avec chiffres avant/après.
- [ ] Fusion `integration/tw2`, commit final.

## Réglage (br3, générique / Paris / Rouen)
- max_blockers 2, dernier carré fort : 10 / 8 / 6
- max_blockers 0, dernier carré fort (0,6 / 0,25 / 0,5) : 2 / 0 / 0
- max_blockers 1, dernier carré fort : 8 / 8 / 3
- max_blockers 0, sans dernier carré : 7 / 2 / 5
- max_blockers 0, dernier carré doux (0,8 / 0,1 / 0,75), engage 45, tireurs qui convergent : 4 / 0 / 3
Paris : la milice assaillante arrive seule, fatiguée et entamée par la chaleur, et rompt au contact.

## Prochaine étape
Assaut groupé (l'assaillant attend d'être assez nombreux dedans avant de marcher sur la place), puis re-mesure.
