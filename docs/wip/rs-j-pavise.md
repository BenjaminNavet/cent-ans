# RS-J — Pavois face à une cible cachée

Branche `feat/rs-j-pavise` (depuis `main`). Chantier RS (`docs/wip/restes.md`), point ouvert de
RS-D (`docs/wip/revue-code.md`, n° 2 : les pavois attendent une cible cachée).

## Règle (core, `sim-battle`)
- `start_attack` (`src/sim/queue.rs`) : les pavois ne restent levés que si la cible est à portée
  **et visible** (`visible` : forêt, murs, crête) ; sinon ils tombent et le régiment marche.
- Boucle de mouvement (`src/sim.rs`) : un régiment à pavois levés dont la cible est à portée mais
  cachée (le cas « visible » tire plus haut) baisse ses pavois et se rapproche comme les autres
  tireurs, puis tire dès qu'il la voit. Hors de portée, il attend toujours derrière ses pavois
  (comportement inchangé). La capacité se termine alors par `tick_abilities` (recharge normale).
- Aucun paramètre nouveau (pas de données).

## Tests (`core/crates/sim-battle/tests/review_fixes.rs`)
- `pavised_crossbowmen_close_in_on_a_hidden_target` activé.
- `pavised_crossbowmen_hold_and_shoot_a_visible_target` (non-régression : cible à découvert, pavois
  toujours levés, ne bouge pas, tire).

## Sondes (release)
| Sonde | Avant | Après |
|---|---|---|
| ep7 Crécy, Anglais 1-20 / 1-30 | 18/20, 27/30 | 18/20, 27/30 |
| ep7 Azincourt | 18/20, 26/30 | 18/20, 26/30 |
| ep7 Poitiers | 16/20, 22/30 | 16/20, 22/30 |
| ep9b_duel `survey` (attaquant) | 4/10 | 4/10 |
| `ai_beats_a_passive_ai_at_equal_forces` | ok | ok |
| eq7 `probe_mixed_battle` (France) | 58/64 | 58/64 |

## État
- [x] Squelette, sondes avant, correction, tests.
- [x] Sondes après : identiques (le cas pavois levés + cible cachée ne survient pas dans ces batailles d'IA).
- [x] `git merge main`, fmt + clippy -D warnings + `cargo test` complet verts ; `core/target-rs-j` supprimée.

Lot terminé, prêt à fusionner.

## Prochaine étape
Fusion par l'orchestrateur RS ; retirer le point n° 2 de `docs/wip/revue-code.md` à la fusion.
