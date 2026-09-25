# Lot R2b — l'IA de bataille lit le relief — terminé (non fusionné dans main)

Branche : `worktree-agent-a0839ed779ec029d6`. Suite de R2 (`docs/wip/r2-relief-bataille.md`, ADR 0020).

## Mesure
`cd core && cargo test --release -p sim-battle --test ai_relief -- --ignored --nocapture`
(`survey_active_against_passive` : IA active contre camp passif, armées miroir, graines 0-15 × 2 camps
= 32 batailles par terrain ; `R2B_SEEDS=0..64` pour un échantillon plus large, `R2B_TIMES=1` pour les
durées ; `trace_one_battle` avec `R2B_TRACE=plains,0,defender` pour suivre une bataille).

| Terrain | avant R2 (c5eb8861) | main après R2 (57032d45) | main (2e428a26, BV2) | R2b (fusionné avec 2e428a26) |
|---|---|---|---|---|
| plaine, graines 0-15 | 21/32 | 17/32 | 19/32 | 29/32 |
| bocage | 11/32 | 9/32 | 9/32 | 23/32 |
| collines | 14/32 | 16/32 | 17/32 | 21/32 |
| montagne | 14/32 | 18/32 | 18/32 | 22/32 |
| plaine, graines 0-63 | — | 76/128 | 78/128 | 113/128 |
| bocage | — | 49/128 | 50/128 | 89/128 |
| collines | — | 80/128 | 81/128 | 98/128 |
| montagne | — | 58/128 | 58/128 | 82/128 |

Ablations avant fusion (graines 0-63, total des 4 terrains sur 512 ; référence 373) : sans « tenir
les hauteurs » 356, sans « course sous les flèches » 348, sans « la ligne attend ses retardataires »
364 ; position de tir, pas de contournement et « pas de charge en
montée raide » : dans le bruit (± 5 ; le seuil de montée raide est passé de 0,12 à 0,20, 0,12 coûtait
6 batailles en montagne).

## État
- [x] `core/crates/sim-battle/src/relief_ai.rs` : `ReliefMap` (proéminence locale par table de sommes,
  pente, montée, coût de marche, ligne de vue, contre-pente), cache `BattleSim::relief_map`
  (`OnceCell`, remis à zéro par `field_mut`).
- [x] `ai.rs` : crête + glacis (pas d'escarpement) pour la position défensive, cherchée depuis la ligne
  de déploiement ; contre-pente de la ligne face aux arbalètes ; défenseur nettement plus haut qui
  garde ses hauteurs ; tireurs vers une position d'où ils voient la cible, plus haute, hors de portée
  des tireurs ennemis ; pas de la ligne décalé pour contourner une montée raide ; pas de charge au pas
  de course en montée raide de loin ; ligne qui court sous les flèches et attend ses retardataires ;
  cavaliers qui ne poursuivent ni ne contournent devant des pieux plantés.
- [x] Fusion de `main` (2e428a26), `cargo fmt`/`clippy -D warnings`/`cargo test` verts,
  `core/build.sh` puis smoke Godot : 24 « smoke OK », 0 erreur.
- [x] Tests : `tests/ai.rs` revenu aux graines 0-2 ; `tests/ai_relief.rs` (seuils statistiques par
  terrain, lecture du relief sur une crête synthétique, contre-pente, défenseur qui garde ses
  hauteurs) ; empreintes de `tests/b6.rs` mises à jour (mêmes vainqueurs).

## Points ouverts
- Haies et village de B5 : la position défensive les prend déjà (B6, `defensive_cover`) avant le
  relief ; pas de score combiné couvert + crête.
- Seul le camp défenseur « tient les hauteurs » ; un attaquant plus haut avance toujours.
- La contre-pente ne sert que contre les arbalètes (les volées d'arc long ignorent la ligne de vue).
- Le gain le plus net vient des cavaliers qui ne s'empalent plus sur les pieux en poursuivant une
  déroute ou en contournant un flanc (première cause de défaite relevée dans les traces), puis de la
  ligne qui avance groupée ; ce ne sont pas des lectures du relief à proprement parler.
- Mesures sur une machine chargée : seules les victoires comptent (déterministes), pas les temps.
