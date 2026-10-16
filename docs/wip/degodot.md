# degodot — retrait du client Godot (17/10/2026)

ADR 0366. Branche bm/degodot (base 9395b327a).

## Fait
- ADR 0366 + index.
- Supprimés : game/{scripts,tests,shaders,scenes,tools,bin}, project.godot, export_presets.cfg, ~4 810 `.import` / `.gdignore` / `.uid`.
- Rust : crate godot-bridge, core/build.sh, build-windows.sh, checks/campaign_sim_check.gd, membre du workspace, profil dev, Cargo.lock régénéré.
- CI windows.yml : job `smoke` Godot retiré, tests du lanceur dans le job `bevy` (Windows), `promote` needs [bevy].
- tools : scripts godot_*/export_*/run_godot_tests/check_gd/sync_godot_version/dn_forest_bench, proto_bevy/godot et bench ; compare_views.sh et montage.py côté Bevy seul ; launch.sh Bevy seul ; .mcp.json sans serveur godot.
- Python : écriture des `.import` retirée (icons, ink_icons, ars_nova, texture_factory, blender_scripts), commande `art models-textures-fix` et texture_import_rules supprimées, tests Godot-only supprimés, typo_apostrophes sans GDScript.
- Lanceur : plus de téléchargement du relief réel v3 ; vérification de `data/map/pyramid/relief_pyramid.json`, lien vers `~/dev/cent-ans-raw/compose/pyramid` s'il existe, sinon commande `cent-ans geo compose-relief-real` ; tests pytest ajoutés.
- Docs : CLAUDE.md, README, CONTRIBUTING, architecture, client/README, manuel, notes historiques.

## Gardé
- game/assets, game/resources/*.tres (lus par Bevy ; forest_detail et map_prop_scale non lus), game/icon.*.
- docs historiques (ADR, wip, godot-map.md…) avec en-tête « Historique » pour les docs de premier niveau.

## Restes
- Convertir game/resources/*.tres en JSON sous data/ (chargeurs client/src/campaign/terrain/tres.rs).
- Commandes `geo relief-fetch/relief-update` et paquet réel v3 : plus utilisées par le lanceur, outils conservés.
- Commentaires « port de game/scripts/… » dans client/src : gardés comme références (historique git avant 9395b327a).
