# Lot B6 — IA tactique et terrain de site

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Branche du worktree B6.

## Objectif
1. L'IA de bataille exploite le site (B5) : tireurs défensifs derrière haie/fossé/clôture ou au bord
   du village, ligne de mêlée adossée à l'obstacle, cavalerie qui évite de charger à travers un obstacle
   qui brise la charge.
2. Lisibilité : ligne de site compacte (« Sol enneigé · hiver · village · haies · côte ouest ») dans le
   dialogue d'avant-bataille et le HUD de bataille.

## Mesures de référence (avant B6)
- Démo 1337 : contact à 70,6 s ; ferme à x = 1117 (bord est), 8 haies/clôtures autour.
- Bataille sans site (plaine, `village = Some(false)`), graine 3 : `238 Some(Attacker) [27, 46, 48, 100, 100, 20, 51, 83, 84, 6]` ;
  graine 11 : `228 Some(Attacker) [23, 47, 47, 100, 100, 25, 65, 84, 86, 4]`.

## État
- [x] Squelette : `core/crates/sim-battle/tests/b6.rs` (sonde, références).
- [ ] IA : position défensive (`ai.rs`).
- [ ] IA : cavalerie qui contourne ou attend.
- [ ] Tests Rust.
- [ ] Pont + UI (dialogue, HUD).
- [ ] Captures `docs/img/b6/`.

## Prochaine étape
Implémenter `cover_position` dans `ai.rs`.
