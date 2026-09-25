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
- [x] Mesures finales (reportées dans l'ADR 0046), fusion de main (c47a54ce puis 848119b5).
- [x] Après fusion : `cargo fmt --check`, `clippy --all-targets -D warnings`, `cargo test --release`
  verts ; `core/build.sh` OK ; test Python du schéma vert.
- Smoke : avec main c47a54ce, 17 « smoke OK » dont bataille, déploiement et siège ; seuls échecs
  « music playlist too short » (lot musique cb1b4416, connu sur main). Avec main 848119b5, le smoke
  plante à l'étape campagne (« Message queue out of memory », code 138, 3 OK) : régression connue de
  main (signalée dans 848119b5, présente sans R3 ni R4), les étapes de bataille ne sont pas atteintes.

## État : terminé (non fusionné dans main)

## Mesures
Voir ADR 0046 § Mesures. R2b 0-63 : 114/88/102/97 = 401/512 (référence 382). Contre-pente : pertes au
trait de la ligne avant contact 88,1 → 17,4 par bataille.

## Points ouverts
- Rendu : `indirect` exporté par `get_shots()` mais pas encore dessiné (cloche plus haute).
- Crête arrondie sans haie : les archers tiennent le sommet et laissent un angle mort (pas de
  « crête militaire » sur le versant avant).
- À rapport de forces serré (3 chevaliers + 1 homme d'armes), la position anglaise crête + haie perd
  désormais : les arcs ne tirent plus à travers la crête.
