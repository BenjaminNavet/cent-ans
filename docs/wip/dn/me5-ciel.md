# ME5 — atmosphère de la carte (branche dn/me5-ciel)

État : implémenté, données + schéma + tests ; reste le jugement visuel de l'orchestrateur.

- Code : `game/scripts/map/map_atmosphere.gd` (créé par `CampaignWeatherView.setup`, `refresh`, `update_view`),
  shaders `game/shaders/map_atmo_*.gdshader` + `map_atmosphere_common.gdshaderinc`.
- Données : `data/fx/map_atmosphere.json` (schéma `fx_map_atmosphere.schema.json`, test pytest). `--no-me5` éteint.
- Familles : cartes de cumulus éclairées (soleil de la carte), cirrus (plan à 96), rideaux de pluie
  (provinces pluie/orage, MultiMesh), bancs de brouillard sur les fleuves (levés avec la brume TB6),
  aurore au bord nord (hiver, plancher de jour, pleine au soir doré). Fumées de villes/feux : déjà `LifeEffects`.
- Tests : `game/tests/me5_atmosphere_test.gd` ; captures : `game/tests/me5_shot.gd` (via tools/godot_bg.sh).
- Prochaine étape : réglage visuel (taille/opacité des cartes, cirrus peu visible, aurore non vérifiée à l'œil),
  alignement des cartes sur les ombres de cumulus RV-C (non fait : positions indépendantes), banc de frame.

## Reprise 2 (retours orchestrateur)
- Cumulus : cartes isolées remplacées par un champ lu dans le MÊME bruit que les ombres RV-C (`cs_cumulus_field`, refactor sans changement de `cs_cumulus`), trois plans empilés, bord effiloché, base sombre, sommet éclairé côté soleil, bancs par le champ régional RV-C (`bank_band`). Nuage et ombre se correspondent.
- Cirrus : opacité 0,38 -> 0,09, fibres fines et plages discontinues, gauchissement réduit, s'efface dès 750-1050.
- Aurore : NON VÉRIFIÉE. Diagnostic : la caméra de jeu (pitch ~40 deg) ne voit le ciel qu'à la limite haute de l'écran ; la rampe de couleur a été renforcée (x22) mais le rendu n'a jamais pu être confirmé (captures noires/bloquées, fence Metal). Piste : afficher le test coloré (R=k, G=band, B=patches) pour trouver pourquoi k paraît nul.
