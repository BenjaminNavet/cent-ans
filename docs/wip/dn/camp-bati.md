# DN camp-bati : glb générés sur la carte de campagne (D1)

Branche `dn/camp-bati`. Voie d'affichage des glb générés semi-réalistes pour les lieux, sous `near_distance`, la maquette stylisée (GC2) gardant le lointain.

- Données : `data/art/dn_campaign_models.json` (`table[type][famille|*] -> [{path, native_width?, scale?, near_distance?, banner?}]`), schéma `art_dn_campaign_models.schema.json`. Table livrée vide = comportement inchangé.
- Code : `game/scripts/map/dn_campaign_models.gd`, extension de `town_maquette_layer.gd` (section « Glb générés »).
- Test : `game/tests/dn_campaign_models_test.gd` (bouche-trous `props_ga/ga3_house_lod1`, `ga3_church_lod1`).
- Brancher un modèle : déposer `game/assets/models/dn/<cat>/<nom>.glb`, importer, ajouter `{"path": "dn/<cat>/<nom>"}` dans la table.

État : voir commits `wip(dn-camp-bati)`.
