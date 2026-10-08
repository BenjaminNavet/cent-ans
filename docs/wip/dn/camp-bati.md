# DN camp-bati : glb générés sur la carte de campagne (D1)

Branche `dn/camp-bati`. Voie d'affichage des glb générés semi-réalistes pour les lieux, sous `near_distance`, la maquette stylisée (GC2) gardant le lointain.

- Données : `data/art/dn_campaign_models.json` (`table[type][famille|*] -> [{path, native_width?, scale?, near_distance?, banner?}]`), schéma `art_dn_campaign_models.schema.json`. Table livrée vide = comportement inchangé.
- Code : `game/scripts/map/dn_campaign_models.gd`, extension de `town_maquette_layer.gd` (section « Glb générés »).
- Test : `game/tests/dn_campaign_models_test.gd` (bouche-trous `props_ga/ga3_house_lod1`, `ga3_church_lod1`).
- Brancher un modèle : déposer `game/assets/models/dn/<cat>/<nom>.glb`, importer, ajouter `{"path": "dn/<cat>/<nom>"}` dans la table.

État : voir commits `wip(dn-camp-bati)`.

## Fait (08/10)
- Fondu croisé par tuile : glb généré opaque sous `near_distance` (défaut 300), maquette dessous jusqu'à l'opacité pleine, maquette seule au-delà de `near + fade_margin` ; portée du type respectée ; lieux sans glb (ou fichier absent) inchangés.
- Bannière procédurale (hampe + toile, `maquette_banner.gdshader` dupliqué, onde de vent) au sommet, MultiMesh par tuile, couleur du contrôleur via `refresh`.
- Test vert : `godot --headless --path game --script res://tests/dn_campaign_models_test.gd -- --no-tb3`. Smoke vert.

## Points ouverts
- Sans `--no-tb3`, les scripts de test isolés bloquent sur `OutbuildingLayer._finish_warm` (attente de la tâche de préchauffe) ; idem pour `gc_maquettes_test` sur main, antérieur à ce lot. `gc_maquettes_test` échoue aussi sur main (« selection ring around the maquette »), indépendant.
- Ombres de nuages RV / étalonnage : glb en matériau standard, éclairage et brume globaux ; pas de lien avec les ombres de nuages du sol.
- Capture `*_shot.gd` non faite (rendu factice en headless).
- Manifeste `data/art/dn_manifest.json` : format du lot dn_ingest inconnu ; le branchement lit directement `table` (une ligne par modèle).
