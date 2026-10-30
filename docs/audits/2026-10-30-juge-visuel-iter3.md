# Juge visuel frais, boucle juge 2, itération 3 (30/10/2026)

État jugé : `fusion-a` (c3db29c39, future main). Client Bevy `--features dev`, graine 1337, France, printemps 1337, temps clair.
Captures : 10 vues de `tools/refshots/views.txt` via `tools/compare_views.sh` (hors écran, muet), lues à 640 px, non commitées :
`camp_2600`, `camp_0600`, `camp_0200`, `camp_0060`, `camp_0025`, `camp_guyenne`, `camp_calais`, `vis_r3b_normandie`,
`vis_r3b_loire`, `battle_default`. Montagne et neige (VIS-R4 en cours) non capturées volontairement.
Mesures : moyenne sRGB puis HSV sur des boîtes de 640 px, script jetable (bandes haut / milieu / bas et zones nommées).

## Note

| Domaine | Note | En une phrase |
|---|---|---|
| Carte de campagne 3D (d 25-600) | **4/10** | Les maquettes Meshy (Paris, Tours) sont au niveau ; tout ce qui les entoure (sol, habitat, horizon) les trahit. |
| Parchemin (d 2600) | 5/10 | Lisible, mais bruyant : moisissure de glyphes, frontières néon, fleuves en cordes. |
| Bataille (vue par défaut) | 6/10 | Ciel nuageux réussi ; sol délavé de terrain de golf, chevaliers en clones gris. |
| UI des plaques | 6/10 | Plaques de lieux belles et lisibles ; plaques d'armées nues, collisions avec le HUD, fantômes à l'horizon. |
| **Global** | **4,5/10** | On est sorti du « plateau gris » (ciel, frontières en bande, eau), mais pas encore entré dans la « photo aérienne peinte ». |

Ce qui a progressé depuis l'audit du 29/10 : le lavis politique a quitté la vue 3D (G1-4 réglé), un ciel existe en vue basse (G1-1
réglé à d ≤ 80), la mer a une bathymétrie et perd son liseré cyan, les parcelles typées (VIS-R3b) donnent des lanières à d 25-80.

## Mesures contre la bible de palette (section c de l'audit du 29/10)

| Zone (vue) | Mesure h / s / v | Cible bible h / s / v | Écart |
|---|---|---|---|
| Plaine, pré (camp_0600) | 67° / 0,20 / 0,50 | pré 89° / 0,56 / 0,50 | saturation ÷ 2,8 |
| Plaine (camp_0200) | 90° / 0,20 / 0,41 | 89° / 0,56 / 0,50 | saturation ÷ 2,8, trop sombre |
| Bocage normand (vis_r3b_normandie) | **123°** / 0,21 / 0,48 | 89° / 0,56 / 0,50 | **teinte +34° vers le bleu-vert (menthe)** |
| Lanières (camp_0060 / camp_0025) | 88-102° / 0,22 / 0,33-0,41 | blé 82° / 0,57 / 0,53 ; labour 32° / 0,44 / 0,62 | aucun labour brun clair visible : 0 % de parcelles au-dessus de v 0,45 |
| Val de Loire (vis_r3b_loire) | 95-97° / 0,18-0,21 / 0,36-0,43 | idem + vigne 60° / 0,40 / 0,40 | pas de vigne lisible |
| Forêt (camp_0600 / camp_0200) | 74-78° / 0,34-0,36 / 0,39-0,40 | 93° / 0,54 / **0,24** | **v forêt / v pré = 0,80-0,95 (cible 0,48)** : la hiérarchie de valeur est cassée |
| Landes (camp_guyenne) | 57° / 0,20 / 0,40 | lande 40° / 0,45 / 0,55 | grise et sombre |
| Seine (camp_0200) | 100° / 0,16 / 0,35 | eau ≈ 200° / ≥ 0,3 | fleuve gris-vert, se lit comme une route |
| Gironde (camp_guyenne) | 101° / 0,16 / 0,55 | estran 150° / 0,29 / 0,42 | coin clair à bords droits |
| Manche, large (camp_0600, normandie) | 183-196° / 0,15-0,22 / 0,37-0,39 | 201° / 0,73 / 0,22 | trop claire, trop grise |
| Brume d'horizon (camp_0060 / vis_r3b_loire) | 211-214° / 0,13-0,18 / 0,80-0,82 | horizon de ciel | occupe 20-25 % du cadre par temps clair |
| Toit de ferme (camp_0025) | 23° / 0,46 / 0,39 | tuile 17° / 0,64 / 0,55 | bon ton, mais rare ; les maisons lointaines sont grises (s < 0,12) |
| Sol de bataille près / loin | 80° / 0,24 / 0,53 ; 59° / 0,18 / 0,58 | pré 89° / 0,56 / 0,50 | délavé, loin presque blanc-jaune |
| Rang de chevaliers (bataille) | 250° / **0,04** / 0,47 | textile héraldique s ≥ 0,25 | gris achromatique |

Constat transversal : **la saturation du sol plafonne à 0,20-0,24 dans toutes les vues 3D**, quel que soit le biome. La chaîne
carte de couleur → `season_ground` → `campaign_post.json` (`aerial.desaturate 0.4`, `grade`) écrase le chroma avant la lumière ; la
bible n'est respectée qu'en valeur. Un sol à s 0,2 est le « gris » que le joueur voit toujours, même sous un ciel bleu.

## Défauts

### G1 (flagrant)

**G1-1. Sol « peigné » à mi-distance, facettes plates au plus près, coutures de tuile.**
- d 150-400 (`camp_0200`, `camp_calais`, `camp_guyenne`, bas de `vis_r3b_normandie`) : toute la plaine est couverte de traits courts
  orientés au hasard, comme une fourrure brossée. Ce sont les lanières de VIS-R3b sous-échantillonnées : `v2b_parcels` bascule sur
  les bandes dès `strip_vis` (3-8 px de cellule), sans préfiltrage analytique du motif à la fréquence de la bande ; le motif
  crénelé devient un hachis directionnel. Aucune photo aérienne ne montre cela.
- d 25 (`camp_0025`) : les lanières sont des aplats polygonaux vert-bleu à bords nets, sans grain ni sillon : un rendu vectoriel qui
  frôle le low poly autour d'une cathédrale Meshy très détaillée.
- `vis_r3b_normandie` : couture rectangulaire franche dans le tiers bas (zone peignée contre zone lisse, Δh 19°, Δv 0,04) et
  ligne horizontale de changement de motif sur toute la largeur. Frontière de palier ou de tuile du sol visible.
- Teinte menthe (h 118-128° en Normandie) et chroma ÷ 2,8 (tableau). La Normandie ne se lit pas comme du bocage : pas de grain de
  haies, pas de prés vifs ; la Beauce et le Val de Loire ont la même texture.

**G1-2. Habitat en pions géants gris.**
- `camp_0200`, `camp_calais`, `camp_guyenne` : des maisons isolées, chacune de 1 à 2 km d'emprise apparente, grises (s < 0,12),
  semées en grappes lâches sur toute la plaine. Elles écrasent Paris (moins haute que trois fermes voisines à d 200) et se lisent
  comme des pions de jeu de plateau, exactement l'anti-cible.
- `camp_0025` : potager en prismes verts extrudés (s 0,36, bords nets) et arbres de jardin en boules lisses : géométrie bâtie par
  le code (`towns/shapes.rs` « garden trees », modèles de champ ADR 0222). Low poly de fait, interdit.
- Figures (paysan, chariot) à l'échelle d'un quartier de Paris.

**G1-3. Horizon fantôme.**
- `camp_0060`, `camp_0025`, `vis_r3b_loire` : au-dessus de la brume flottent des cavaliers translucides, des bannières d'armées,
  des étiquettes de fleuves et de pays à 200-400 km (« Tamise » et « Seine » vues depuis Tours ; « Escaut », « Meuse », « Rhin »
  depuis Paris). Les plaques et étiquettes ne sont ni occultées par la brume, ni coupées par la distance d'horizon.
- La brume d'horizon (v 0,80, s 0,13-0,18) occupe 20-25 % du cadre par temps clair, avec des taches grises (arbres lointains) qui
  y flottent. La bible fixe « jamais au-delà de 35 % en vue moyenne » ; ici c'est en vue basse, avec un ciel sans nuage au-dessus :
  la ligne d'horizon est une bande de lait, pas un paysage qui s'efface.

**G1-4. Gironde en coin clair à bords droits** (`camp_guyenne`). Défaut connu depuis la migration Bevy, toujours là : un triangle
turquoise-pâle rectiligne de Royan à Bordeaux, déconnecté de la Garonne et de la Dordogne, qui arrivent en rubans sombres.

### G2 (majeur)

- **G2-1. Forêts en mousse claire.** Masses vert-jaune duveteuses, à peine plus sombres que les champs (v 0,39-0,40 contre
  0,41-0,50) ; bord flou ; reflet jaune sur le dessus. La bible veut v 0,24 et une lisière nette (Cassini).
- **G2-2. Fleuves sans largeur ni eau.** La Seine et la Loire sont des fils gris-vert sombres (h 100°, s 0,16), sans berge claire,
  sans bancs de sable (la Loire est le fleuve à grèves par excellence), sans reflet de ciel. À d 200 ils se lisent comme des routes.
- **G2-3. Parchemin (d 2600) bruyant.** Pointillé de glyphes d'arbres noirs sur tout le continent (G3-2 du 29/10 inchangé) ;
  frontières en double trait néon jaune-vert-rouge-blanc ; réseau fluvial en cordes bleu sombre de 4-6 px qui domine le dessin ;
  royaume de France en lavis gris-mauve (s 0,15) plus terne que tous ses voisins.
- **G2-4. Plaques d'armées nues.** « 620 », « 240 », « 100 » : rectangles blancs sans blason, sans cadre, sans couleur de position,
  étrangers au style des plaques de lieux (cadre or, écu, pastilles). À d 2600 on ne sait pas à qui sont les armées.
- **G2-5. Plaques sous le HUD.** Dublin, York, Édimbourg, Péronne coupées par la barre du haut ou le Journal ; aucune zone sûre.
  Colonne gauche (Journal, Récit, Audience, chroniqueur) : 24 % de la largeur sur toute la hauteur au premier écran ; le monde
  utile est réduit au centre droit.
- **G2-6. Mer terne.** Manche s 0,15-0,22, v 0,37-0,39 contre s 0,73, v 0,22 : grise et laiteuse, conséquence du même
  désaturateur aérien que le sol. La falaise de craie de Blanc-Nez est un trait blanc continu sans relief.
- **G2-7. Bataille : sol délavé, chevaliers clones.** Sol v 0,53-0,58, s 0,18-0,24, presque uniforme jusqu'à l'horizon ; rang de
  chevaliers achromatique (s 0,04), mêmes silhouettes répétées : lecture « coussins gris ». Les archers du fond, eux, sont variés.
- **G2-8. Villes sans maquette** à d 300-400 : Bordeaux et Caen n'ont qu'une plaque (à vérifier : seuil de visibilité ou
  occlusion par la plaque).

### G3 (moyen)

- Ciel de campagne sans nuage (la bataille en a de très beaux) : dégradé uni, aucune lecture de météo ni de vent.
- Bataille : colonne blanche verticale fine à gauche du moulin (fumée réduite à un trait ?).
- Bataille : ligne d'arbres d'horizon en bande plate continue, sans profondeur.
- Montagne et neige : non jugées (VIS-R4 en cours) ; à re-juger après fusion.
- Enclave de Calais (camp_0600) : contour rouge vif autour d'un territoire minuscule, plus voyant que les vraies frontières.

## Lots de refonte proposés

Ordre : L1 et L3 d'abord (ils touchent tout le cadre) ; L2 ensuite ; L4-L6 en parallèle. L1 et L4 touchent `terrain.wesl` : les
sérialiser ou découper le shader d'abord (L1, étape a).

### L1 SOL-PHOTO : bake du sol en clipmap + chroma rendu (L, Opus pour la conception)
- Spec. (a) Découper `terrain.wesl` (2 900 lignes) en modules (`ground_albedo`, `parcels`, `seasons`, `parchment`, `water_edge`).
  (b) Nouvelle architecture : l'albédo du sol (carte de couleur + parcelles typées + saisons par classe) est **cuit dans une clipmap
  de 3-4 niveaux** (2048², RGBA8, centrée caméra, rafraîchie par anneaux quand la caméra bouge) au lieu d'être recalculé par
  fragment et par image ; le fragment final ne fait que lire la clipmap, ajouter le micro-détail et la lumière. Le préfiltrage
  devient gratuit (mips de la clipmap) : fin du peigné. (c) Lanières : sillons et chaume en texture de détail Z-Image (tuileable,
  8 cultures × 2 saisons) au lieu d'aplats ; bord de parcelle légèrement irrégulier. (d) Chroma : retirer la désaturation verte de
  `grade` et réduire `aerial.desaturate` à ≤ 0,15 hors lointain ; fixer la saturation dans la carte de couleur, pas en post.
- Critères. Plaine h 80-95°, s ≥ 0,42 sur `camp_0600` / `camp_0200` ; au moins 15 % de pixels de labour / chaume v ≥ 0,50 à d 60 ;
  v forêt / v pré ≤ 0,6 ; aucune couture : Δh < 5° de part et d'autre de toute frontière de palier (test de capture sur
  `vis_r3b_normandie`) ; énergie de gradient directionnel à 4-12 px sur `camp_0200` divisée par 3 (mesure scriptée).
- Fichiers : `client/src/campaign/terrain/{terrain.wesl, fields.rs, parcel.rs, ground.rs, post.rs}`, nouveau
  `terrain/albedo_bake.rs`, `data/fx/campaign_post.json`, `data/art/ground_biome_mix.json`, `tools/` (carte de couleur), ADR.
- GPU : bake ≈ 0,5 ms sur les images de déplacement, gain de 1-2 ms en régime (le parcellaire n'est plus évalué par fragment).
  Mémoire +64 Mo (4 × 16 Mo).
- Assets : Z-Image local, 16 textures de détail de culture + 2 de haie bocagère. Aucun Meshy.

### L2 HABITAT-À-L'ÉCHELLE : noyaux villageois ancrés, fin des pions (M)
- Spec. Supprimer les maisons isolées géantes au-delà de d 120 ; un hameau devient un **noyau** : décalque au sol (jardins,
  vergers, halo de terre battue, 3-6 toits rouges) + au plus une maquette Meshy existante (`village_west`) à l'échelle ≤ 3 × réel.
  Remplacer les arbres de jardin et potagers bâtis par le code par les billboards / instances d'arbres déjà en place. Échelle des
  figures et chariots plafonnée relativement aux maquettes de ville. Vérifier la visibilité des villes (Bordeaux, Caen).
- Critères. Aucune maison isolée > 400 m d'emprise à d ≥ 150 ; toits de village h 10-30°, s ≥ 0,40 (taches chaudes de la bible) ;
  aucun maillage bâti par le code dans `towns/shapes.rs` et la pose des champs (test grep + test d'inventaire) ; Paris plus haute
  que tout bâtiment voisin à d 200 ; toute ville `city` visible à d ≤ 1 000.
- Fichiers : `client/src/campaign/towns/{shapes.rs, plan.rs, layer.rs}`, `countryside/{field_plan.rs, decor.rs, decor_plan.rs}`,
  `data/art/town_maquettes.json`, `data/map/map_countryside.json`.
- GPU : neutre à négatif (moins d'instances). Assets : Z-Image local pour 4 décalques de noyau (openfield, bocage, Midi, Nord) ;
  Meshy seulement si un `village_south` manque (≤ 20 crédits, à décider après mesure).

### L3 AIR-HORIZON : brume unique, nuages, coupe des plaques lointaines (M)
- Spec. Une seule passe d'air sur la profondeur pour terre et mer (G2-3 du 29/10, toujours à finir). Couche de nuages de
  campagne (reprendre les nuages de bataille sur le dôme). Les plaques, étiquettes, bannières et maquettes d'armées dont la
  densité de brume dépasse 0,6, ou au-delà de la distance d'horizon, sont masquées ; l'étiquette ne flotte jamais au-dessus du sol
  qu'elle nomme.
- Critères. Par temps clair, bande de brume ≤ 12 % du cadre à d 60-80 ; bande haute h 205-220°, s ≥ 0,25 ; zéro plaque, étiquette
  ou figure d'armée rendue au-dessus de la ligne d'horizon (test dans le dump d'état : projection écran contre ligne d'horizon) ;
  mer et terre de même transmittance à profondeur égale (± 5 %).
- Fichiers : `client/src/campaign/terrain/post.{rs,wesl}`, `weather/{dome.rs, atmosphere.rs}`, `water/shaders/sea.wesl`,
  `markers/`, `armies/`, `data/fx/{campaign_post.json, campaign_sky.json}`.
- GPU : +0,3 ms (nuages), compensé par la suppression de la brume au fragment du sol. Assets : aucun (nuages déjà en bataille).

### L4 EAU-QUI-BRILLE : fleuves larges, Gironde, mer saturée (M)
- Spec. Largeur de fleuve en mètres réels (Loire 300-500 m, grèves de sable à d ≤ 100), berge claire et ripisylve, reflet du
  ciel ; Gironde reconstruite depuis le trait de côte (estuaire, vasières) ; mer suivant la bible (large s ≥ 0,6), falaises de craie
  en bande ombrée.
- Critères. Fleuve h 195-215°, s ≥ 0,30 à d 200 ; Gironde sans arête rectiligne > 20 km ; Manche large s ≥ 0,55, v ≤ 0,28.
- Fichiers : `client/src/campaign/water/{shaders/river.wesl, river_fine.wesl, sea.wesl}`, données de fleuves et de côte dans
  `data/map/`, `data/fx/sea_life.json`. GPU : +0,2 ms. Assets : aucun.

### L5 PLAQUES-ET-PARCHEMIN : armées héraldiques, zone sûre, parchemin sobre (S-M)
- Spec. Plaque d'armée au style des plaques de lieux (écu du chef, couleur de position, effectif) ; zone sûre : aucune plaque sous
  le HUD (repli ou masquage) ; colonne gauche repliée par défaut après le premier tour. Parchemin : glyphes d'arbres ÷ 3, frontière
  en bande unique de 4-8 px sans double néon, fleuves majeurs ≤ 3 px et mineurs ≤ 1 px, lavis de la France à la couleur de sa
  faction.
- Critères. Parchemin : pixels sombres (v < 0,25) ≤ 8 % de la terre ; saturation des frontières ≤ 0,6 ; zéro plaque tronquée par
  le HUD (test d'UI) ; HUD au repos ≤ 15 % de la surface.
- Fichiers : `client/src/campaign/{markers, armies, hud, parchment}`, `data/map/stance_cues.json`, `data/ui/`.
  GPU : nul. Assets : aucun.

### L6 BATAILLE-MATIÈRE : sol vivant, chevaliers héraldiques (M)
- Spec. Étalonnage du sol de bataille sur la bible (pré s ≥ 0,40, v ≤ 0,50 au premier plan), variation par touffes, chemins et
  piétinement ; surcots et caparaçons aux couleurs de la bannière du régiment, variation de teinte par homme ; fumée de cheminée
  en volume ; horizon d'arbres en deux plans.
- Critères. Sol de bataille près s ≥ 0,38 ; rang de chevaliers s moyen ≥ 0,25 ; aucune silhouette identique côte à côte (variation
  d'échelle et de teinte par instance).
- Fichiers : `client/src/battle/decor/ground.wesl`, `battle/units/skinned.wesl`, `data/fx/{battle_ground.json, battle_lighting.json,
  unit_looks.json}`. GPU : +0,1 ms. Assets : Z-Image local pour 6 motifs héraldiques de surcot.

## Ce que le prochain juge doit vérifier en premier
1. `camp_0200` et `camp_calais` : plus de peigné, plus de pions géants.
2. `camp_0060`, `vis_r3b_loire` : horizon sans fantômes, brume ≤ 12 %.
3. Saturation de la plaine ≥ 0,42 sur toutes les vues 3D.
