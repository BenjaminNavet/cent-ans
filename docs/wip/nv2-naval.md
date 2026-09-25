# Lot NV2 — Finitions navales

Branche `worktree-agent-a8e48e736810f5f6e`. Suite de NV1 (ADR 0028, complément NV2 ;
`docs/wip/nv1-naval.md`). Captures : `docs/audit/captures/nv2/`.

## État
| Étape | État |
|---|---|
| 1. Coques : bordé à clin, goudron, ferrures, usure, reflet mouillé, châteaux peints patinés | fait : `game/shaders/naval_hull.gdshader` (espace objet, virures, joints d'about, clous rivés et rouille, brai sous la flottaison, bande mouillée animée, algues, crasse, usure, brûlures), `naval_paint.gdshader` (panneaux, liseré, peinture passée et écaillée) branchés dans `naval_ship_view.gd::_dress` ; aucun triangle ajouté, aucune texture externe (rien à créditer) ; caméra de capture `--camera=hull` |
| 2. IA navale : abordages simultanés, lignes enchaînées, répartition des cibles | fait : `naval/ai.rs` (abordage général, répartition `boarders_per_target`, pénalité des cibles masquées), `sim.rs::chain_onward` / `chain_crossings` (passage d'un navire enchaîné pris au suivant), contre-abordage enchaîné, `chain_morale` ; formation `chains` des scénarios (`sluys.json`) ; tests `sim-battle/tests/nv2_naval.rs` ; accord auto/3D 25/26 |
| 3. Noms historiques des navires de campagne | fait : `data/naval/ship_names.json` + schéma, `ShipNamer` (`sim-campaign::naval`), tests `nv2_naval_campaign.rs`, `tools/tests/test_nv2_naval.py` |
| 4. Traversée vers Calais : Manche / pas de Calais | fait : `Settlement::sea_zone` (Calais, Wissant, Boulogne, Douvres), `crossing_sea`, `fleets.json::port_waters` (« le pas de Calais »), écran d'avant-bataille |
| 5. HUD : bandeau des navires | fait : cartes pleines sur une ligne si elles tiennent, sinon compactes sur deux lignes, flèches et molette au-delà ; test `game/tests/nv2_naval_test.gd` (1280×720, 1920×1080) |

## Mesures (l'Écluse, graines 1-3)
Avant : 1 abordage à la fois, 1-2 navires anglais à l'abordage, Français rendus sous les flèches.
Après : 3-5 abordages simultanés, 6-8 navires anglais à l'abordage, 3-11 passages le long des
chaînes, pertes anglaises 105-200 (46 avant), victoire anglaise, galères génoises échappées.

## Prochaine étape
Lot terminé (vérifications après fusion de main). Suites possibles : ordre joueur « Abordage
général » dans le HUD ; normal map du clin (les modèles n'ont pas de tangentes) ; noms de navires
pour les réserves de campagne affichées ailleurs que dans la bataille.
