# Suites ouvertes de la vague 3 « bulles partout » (25/09)

Origine : fin de `docs/wip/bulles-partout.md` (« Suites ouvertes »). Le joueur a demandé de tout corriger.

| Lot | Contenu | Branche | État |
|---|---|---|---|
| SV1 Vision | fusion de `m5a-vision` + portée de vision armée/ville lue depuis les données | sv1-vision | **fait** — `m5a-vision` fusionné sans conflit ; `vision_army_km`/`vision_settlement_km` lus par le cœur (test `sight_radii_follow_the_data`) ; `edge_feather_km` sorti du code vers `data/rules/vision.json` ; liseré du brouillard estompé aux paliers ZG4 |
| SV2 Coûts unités | coûts en ressources des unités prélevés (cœur, pont, UI, IA) | sv2-unit-resources | **fait** — règle B7c/ADR 0053 : tirage réservé pendant la levée (`QueuedRecruit::drawn`), manque importé et payé, refus si trésor insuffisant ; IA recalcule le prix ; 8 fiches codex corrigées ; détail `docs/wip/sv2-unit-resources.md` |
| SV3 Panneau + particules | surcoût d'import dans le panneau de province ; erreur `scale_particles` au smoke | sv3-panel-particles | **fait** — ligne « Dont import » dans `PanelWidgets.fill_buildable` ; cause : nœud de particules libéré avant l'appel différé → on diffère l'ID d'instance |
| SV4 Chiffres en dur | chiffres de règles écrits en dur dans les GDScript → lus depuis le cœur / `data/` | sv4-ui-numbers | en cours |

Fusion : worktree `../gp-suites3-merge` (branche `integration/suites3`), puis ff-only dans main.

## Limites restantes
- L'IA voit toute la carte (choix M5a) ; maquettes, forêts, hameaux et mer non voilés.
- Montants d'import (300/900 livres) écrits en toutes lettres dans le codex : à resynchroniser si les données changent.
- `campaign_sim_mock.gd` n'a pas les champs de ressources des recrues (valeurs par défaut).

## Prochaine étape
Fusionner SV4, smoke + cargo test + pytest sur l'intégration, ff dans main.
