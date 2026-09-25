# Suites ouvertes de la vague 3 « bulles partout » (25/09)

Origine : fin de `docs/wip/bulles-partout.md` (« Suites ouvertes »). Le joueur a demandé de tout corriger.

| Lot | Contenu | Branche | État |
|---|---|---|---|
| SV1 Vision | fusion de `m5a-vision` dans main + portée de vision armée/ville lue depuis les données | sv1-vision | **terminé** (branche prête, non fusionnée dans main) : `m5a-vision` fusionné sans conflit ; rayons `vision_army_km`/`vision_settlement_km` lus par le cœur (déjà branchés par M5a, prouvés par `sight_radii_follow_the_data`) ; bord doux `edge_feather_km` sorti du code vers `data/rules/vision.json` ; liseré du brouillard estompé aux paliers ZG4 ; cargo test (104 binaires), smoke, `m5a_vision_ui_test`, pytest (561) verts |
| SV2 Coûts unités | coûts en ressources des unités réellement prélevés (cœur, pont, UI, IA) | sv2-unit-resources | lancé |
| SV3 Panneau + particules | surcoût d'import de pierre dans le panneau de province ; erreur `scale_particles` en boucle au smoke | sv3-panel-particles | lancé |
| SV4 Chiffres en dur | chiffres de règles écrits en dur dans les GDScript → lus depuis le cœur / `data/` | sv4-ui-numbers | lancé |

Fusion : worktree d'intégration séparé, puis ff-only dans main (voir mémoire « shared index merges »).
Disque : 18 Go libres au lancement ; chaque agent supprime son `core/target` en fin de lot.

## Prochaine étape
Attendre les 4 agents, fusionner dans `integration/suites3`, smoke + cargo test + pytest, ff dans main.
