# PLAQUES-2 — plaques projetées sur la 3D

État : en cours.
- Campagne : `declutter.scene_height_ratio` (data/map/settlement_markers.json) ; silhouette 3D des cités-repère (largeur x ratio) masque le nom des lieux plus lointains projetés dessus (`markers/mod.rs::update_labels`).
- Bataille : `battle.plates_horizon` (data/ui/camera_feel.json, schéma mis à jour) ; réutilise `HorizonRule` ; plaques ennemies lointaines rétrécies puis masquées (`battle/units/markers.rs`). Alliés et sélection exemptés.
Prochaine étape : check/clippy/tests, mesure dump, 2-3 captures.

TERMINÉ : fmt, clippy -D warnings, tests client, smoke 14/14 verts. Aucune capture ni dump visuel (réglages scene_height_ratio 0.5 et plates_horizon 4/300 m non jugés visuellement).
