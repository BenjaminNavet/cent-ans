# bm-models : modèles 3D de la carte de campagne (client Bevy)

Branche `bm/models`, dossiers `client/src/campaign/{armies,markers,life}/`.

## Diagnostic
- Lieux : Godot affiche, sous `near_distance` (1300, au-delà de toute portée de maquette), les glb générés
  DN camp-bati (`data/art/dn_campaign_models.json`, `dn_campaign_models.gd`) à la place des maquettes du kit ;
  Bevy ne montrait que les maquettes du kit, dont la famille ouest (matériau `Building`, atlas non porté)
  sort blanche. Port : `markers/dn.rs` (table, sous-familles, port, échelle, gain d'albédo, bannière).

## État
- En cours : vérification par capture.

## Prochaine étape
- Captures Paris d=60/25/200 ; armées, convois, bateaux, oiseaux.
