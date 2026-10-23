# 0755 — Palette unique, six heures de bataille, voile de campagne réduit

Statut : accepté (23/10). Amende 0097 (préréglages d'heure : trois deviennent six), 0406 (contraste clair de bataille), 0710 (étalonnage et voile de la carte). 0596 (météo brouillard) inchangé.

## Contexte
Juge DA du 23/10 (5,3 / 10) : aube quasi identique au midi, crépuscule jaune-kaki sale à taches noires, campagne gris-vert froid contre bataille jaune-vert et ciel bleu vif, herbe de bataille en « peau de léopard », voile gris-bleu en campagne même par temps clair.

## Décision
- **Bible de palette** `data/fx/palette.json` + `docs/design/palette.md` : saison de référence printemps, heure midi, matières, ciel, étalonnage commun avec tolérances, cibles de mesure. Le sol de bataille (`season_ground`) y est lu ; un test tient campagne et bataille dans la tolérance de l'étalonnage commun.
- **Six heures** (aube, matin, midi, après-midi, crépuscule, nuit) au lieu de trois : soleil par température de couleur (`sun_kelvin`), élévation basse aux extrémités, ciel HDRI « dawn » avec disque, et **air** par heure (`time_of_day.<clé>.air` : brume dorée à l'aube, orangée au soir, visibilité, ambiance, application partielle sous pluie / brouillard / neige). Le changement de phase met à jour soleil, ambiance, couleur de fond et brouillard de la caméra.
- **Taches du crépuscule** : cause = les bosses de détail et normales de couches du sol, amplifiées en lumière rasante (N.L varie de +-60 % à 9° contre +-20 % à 30°). Elles sont atténuées avec la hauteur du soleil dans `ground.wesl` (`LOW_SUN_BUMP`), coût nul. L'ancien soleil du soir (12°, énergie 1,45) laissait en plus le sol à 20 % de la luminance de midi.
- **Herbe** : macro-variation de teinte à trois octaves de bruit monde (un tap de plus), teintes « sèches » moins jaunes.
- **Campagne claire** : lift des noirs ramené à presque zéro (c'était le gris-bleu), saturation 0,95 au lieu de 0,8, désaturation de `campaign_post.json` 0,06 (0,16 / 0,22 avant), brume de vallée claire 0,06, perspective aérienne x0,5 par temps clair (`clear_scale`, remonte à la valeur de 0596 avec le brouillard : `1 - fog_cut`).

## Conséquences
- Aucune passe plein écran ajoutée ; coût : un tap de bruit par pixel de sol. Bench non lancé (charge machine) : à mesurer à l'intégration.
- Campagne : réglages vérifiés par tests et données seulement, le terrain ne se chargeait pas à temps en profil debug sous charge (captures non exploitables).
- Reste : jaune des couches de texture de bataille (teinte 67° mesurée, cible 82-106°), interpolation continue du soleil entre phases (saut aux changements de phase), `horizon`/panorama/volumétrique utilisent encore la brume non teintée par l'heure, cadrage plus ciel en vue haute (caméra, hors lot).
