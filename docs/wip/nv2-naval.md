# Lot NV2 — Finitions navales

Branche `worktree-agent-a8e48e736810f5f6e`. Suite de NV1 (ADR 0028, `docs/wip/nv1-naval.md`).

## État
| Étape | État |
|---|---|
| 1. Coques : bordé à clin, goudron, ferrures, usure, reflet mouillé, châteaux peints patinés | à faire |
| 2. IA navale : abordages simultanés, lignes enchaînées, répartition des cibles | fait (`naval/ai.rs`, `sim.rs::chain_onward`, règles `boarders_per_target`, `assault_*`, `chain_morale`, formation `chains` des scénarios ; tests `sim-battle/tests/nv2_naval.rs` ; accord auto/3D 25/26) |
| 3. Noms historiques des navires de campagne (données + schéma) | à faire |
| 4. Traversée vers Calais : zone maritime Manche / Pas de Calais | à faire |
| 5. HUD : bandeau des navires défilant / compact (1280×720, 1920×1080) | à faire |

## Prochaine étape
Données : noms des navires (3), zone maritime de Calais (4) ; puis HUD (5), coques (1).
