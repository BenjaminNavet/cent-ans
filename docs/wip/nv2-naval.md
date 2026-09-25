# Lot NV2 — Finitions navales

Branche `worktree-agent-a8e48e736810f5f6e`. Suite de NV1 (ADR 0028, `docs/wip/nv1-naval.md`).

## État
| Étape | État |
|---|---|
| 1. Coques : bordé à clin, goudron, ferrures, usure, reflet mouillé, châteaux peints patinés | shaders `naval_hull`/`naval_paint` écrits et branchés (`naval_ship_view.gd::_dress`) ; captures à faire |
| 2. IA navale : abordages simultanés, lignes enchaînées, répartition des cibles | fait (`naval/ai.rs`, `sim.rs::chain_onward`, règles `boarders_per_target`, `assault_*`, `chain_morale`, formation `chains` des scénarios ; tests `sim-battle/tests/nv2_naval.rs` ; accord auto/3D 25/26) |
| 3. Noms historiques des navires de campagne (données + schéma) | fait (`data/naval/ship_names.json`, `ShipNamer`, tests `sim-campaign/tests/nv2_naval_campaign.rs`, `tools/tests/test_nv2_naval.py`) |
| 4. Traversée vers Calais : zone maritime Manche / Pas de Calais | fait (`Settlement::sea_zone`, `fleets.json::port_waters`, `crossing_sea`) |
| 5. HUD : bandeau des navires défilant / compact (1280×720, 1920×1080) | écrit (`naval_hud.gd`), test `game/tests/nv2_naval_test.gd` à passer |

## Prochaine étape
Vérifs complètes (cargo test en cours), build.sh, import, smoke, tests Godot NV1/NV2, captures
fenêtrées (`-- --naval-scenario=sluys`) des coques et du bandeau, ajustement des shaders.
