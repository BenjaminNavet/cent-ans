# Suites ouvertes de la vague 3 « bulles partout » (25/09)

Origine : fin de `docs/wip/bulles-partout.md` (« Suites ouvertes »). Le joueur a demandé de tout corriger.

| Lot | Contenu | Branche | État |
|---|---|---|---|
| SV1 Vision | fusion de `m5a-vision` dans main + portée de vision armée/ville lue depuis les données | sv1-vision | en cours : m5a fusionné, rayons lus depuis `data/movement/rules.json`, bord doux dans `data/rules/vision.json` ; vérifications en cours |
| SV2 Coûts unités | coûts en ressources des unités réellement prélevés (cœur, pont, UI, IA) | sv2-unit-resources | lancé |
| SV3 Panneau + particules | surcoût d'import de pierre dans le panneau de province ; erreur `scale_particles` en boucle au smoke | sv3-panel-particles | lancé |
| SV4 Chiffres en dur | chiffres de règles écrits en dur dans les GDScript → lus depuis le cœur / `data/` | sv4-ui-numbers | lancé |

Fusion : worktree d'intégration séparé, puis ff-only dans main (voir mémoire « shared index merges »).
Disque : 18 Go libres au lancement ; chaque agent supprime son `core/target` en fin de lot.

## Prochaine étape
Attendre les 4 agents, fusionner dans `integration/suites3`, smoke + cargo test + pytest, ff dans main.
