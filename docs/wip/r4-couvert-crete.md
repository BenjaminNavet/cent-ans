# Lot R4 — contre-pente contre l'arc long, position « couvert × crête »

Branche `worktree-agent-a87b717776dc51d1c`. Suite de R2b (`docs/wip/r2b-ia-relief.md`), ADR 0046.

## Référence (main afd327d4, avant R4)
Mesure R2b, graines 0-63 : plaine 113/128, bocage 89/128, collines 98/128, montagne 82/128 (total 382/512).

## État
- [x] Squelette : `data/rules/missile_arc.json` + schéma + test Python ; `src/missile_arc.rs`
  (`MissileArcRules`, `FireMode`, `arc_clears`) ; `src/sim/indirect.rs` (`fire_mode`, observateur) ;
  `Unit::seen_at` ; `ShotEvent::indirect` + pont `get_shots().indirect` ; `src/position.rs` (vide).
- [x] Tir indirect branché (`visible`/`fire` de `sim.rs` : 3 retouches), IA : la ligne recule sur la
  contre-pente contre tout tireur, les tireurs cherchent un poste d'où ils voient.
- [x] Score de position (`position.rs`) + `defensive_ground` (ai.rs) qui réunit B6 et R2b, course
  (position que l'ennemi atteindrait avant nous rejetée), attaquant qui vise le point faible.
- [x] Attaquant plus haut qui attend (150 s au plus).
- [x] Tests `tests/r4.rs` (9, vérifiés en retirant chaque fonction) ; surveys `tests/r4_survey.rs`.
- [x] ADR 0046 (brouillon, chiffres à reporter).
- [ ] Mesures finales (R2b 0-63 + surveys avant/après), fusion de main, build.sh + smoke.

## Mesures intermédiaires
- R2b 0-63 (avant la « course ») : plaine 114, bocage 89, collines 102, montagne 101 (406/512 contre 382).
- Géométrie A (crête 80 m devant le déploiement) : la crête+haie coûtait la bataille (les Anglais
  montaient 140 s et se faisaient prendre en marche) → ajout de la course ; surveys passés à une crête
  45 m devant.

## Prochaine étape
Relire les surveys (avant : copie de afd327d4 dans le scratchpad), remesurer R2b, finir l'ADR.
