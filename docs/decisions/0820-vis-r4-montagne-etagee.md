# 0820 : montagne étagée et neige d'altitude blanche (VIS-R4)

Statut : proposé (lot VIS-R4, boucle juge 2). Numéro réservé.
Amende 0710 et 0750 sur un point : la règle « neige jamais blanc pur » ne vaut plus pour la neige d'altitude. Elle reste pour la neige de plaine.

## Contexte

Constat G2-1 de l'audit visuel du 29/10 : les massifs sont en plâtre rose, sans étages ni neige lisible. Causes : la neige était
volontairement grise (`snow.plain_color` 0,70 0,68 0,64), donc de même valeur que la roche après étalonnage et brume ; aucun
étagement par altitude réelle ; la carte de couleur bascule en roche dès 1 700 m. Contraintes dures : ALU seulement, aucun
échantillonneur de plus (limite Metal de 16), coût visé +0,1 ms, réglages en données avec schéma.

## Décision

1. **Étagement dans le shader du sol** (`terrain.wesl`, `mountain_stages`), en ALU, d'après l'altitude réelle, la pente affichée et
   l'exposition (adret / ubac) : forêt de pente (résineux) jusqu'à la limite des arbres, alpage, éboulis, roche, neige. La limite des
   arbres descend vers le nord (Alpes > Pyrénées > Highlands) et sur les ubacs.
2. **Neige d'altitude blanche** : au-dessus de `mountain.snow.alpine_from_m`, la teinte de la neige passe de `plain_color` (gris chaud,
   inchangé pour la plaine) à `mountain.snow.color` (0,90 0,92 0,95 au soleil, ombre 0,62 0,68 0,80, bible de palette). Amendement
   de 0710/0750 : « jamais blanc pur » vaut pour le blanc 1,0 et pour la plaine ; la neige d'altitude est le blanc du monde.
3. **Plaine d'hiver** : `snow.plain_share` passe de 0,93 à une valeur basse (neige de plaine épisodique), le drap blanc relève de la météo « neige ».
4. **Glaciers** : lignes et couleurs en données (`map_freshwater.json`), langues plus lisibles.
5. **Brume de vallée** : plafond par temps clair abaissé en données (`campaign_post.json`).
6. **Données** : bloc `mountain` de `data/ui/campaign_seasons.json` (schéma `campaign_seasons_ui.schema.json`), lu par `season.rs`
   dans de nouveaux vec4 de l'uniforme `SeasonParams` (aucun échantillonneur).

## Conséquences

- La neige d'altitude du jeu passe de gris à blanc ; la roche, la forêt de pente et l'alpage se lisent par la teinte.
- Aucune règle de jeu touchée ; la carte de couleur cuite n'est pas régénérée.
- Coût : quelques dizaines d'opérations ALU par pixel de sol, dans le cache de sol quand il est actif sinon en direct.
