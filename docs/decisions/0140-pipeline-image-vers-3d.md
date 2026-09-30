# 0140 — Pipeline image-vers-3D pour le décor de bataille (GA3-L1)

Date : 2026-09-30. Statut : acceptée (go du joueur sur les sondes GA3, lot L1).
Notes : `docs/wip/ga3.md` (sondes S1-S5, lot L1), `docs/wip/ga.md` §GA3.

## Contexte
Le décor de bataille (hameaux, église, moulins, camps, mobilier, palissades) vient du kit Blender
BR1 (`game/assets/models/buildings/`, 100 à 1 500 triangles par modèle, matières partagées). Les
sondes GA3 ont montré qu'un modèle généré par image-vers-3D est bien au-dessus du kit pour
≈ 0,05 $ l'objet (S1, S4), que TRELLIS 2 ne vaut pas son prix ×15 pour le décor sauf un
bâtiment-clé (S4), et que les figurines animées ne s'y prêtent pas encore (S2, S3).

## Décision
1. **Chaîne** (catalogue `data/art/ga3_decor.json`, schéma `art_ga3_decor.schema.json`) :
   prompt commun (XIVe siècle français, sans anachronisme, fond gris uni, sans sol) →
   `fal-ai/flux-2` 1024² (0,012 $) → détourage `fal-ai/bria/background/remove` (0,018 $) →
   `fal-ai/trellis` (0,02 $ ; église : `fal-ai/trellis-2` 1024, 0,30 $) →
   `tools/blender_scripts/ga3_cleanup.py` : îlots-débris retirés, emprise alignée sur les axes
   (rectangle d'aire minimale, faîtage sur +X, façade +Z), taille réelle (longueur ou hauteur),
   LOD0/1/2 (100/50/15 % du plafond), pied posé à y = 0 par LOD, albédo recuit et étalonné
   (`--auto-levels 0.5`, pas de gamma : S4 blanchissait le chaume), normal map cuite du maillage
   source. Scripts : `tools/experiments/ga3_fal_decor.py --catalog` (étapes en cache, coût
   consigné) puis `tools/experiments/ga3_decor_build.py` (Blender + `props_ga/manifest.json`).
   Planche : `tools/blender_scripts/ga3_decor_sheet.py`.
2. **Plafonds** : bâtiments LOD0 ≤ 8 k triangles (église 25 k), objets ≤ 3 k.
3. **Pièces fines** : `fal-ai/trellis/multi` (vue source + profil et dos regénérés par
   `fal-ai/flux-2/edit`) a été essayé sur charrette, palissade, trébuchet, bélier ; la vue unique
   donne partout un maillage d'un seul îlot au moins aussi propre (le multi-vue ajoute des débris
   là où les vues regénérées divergent). La vue unique est retenue ; le multi-vue reste dans le
   script pour les objets dont la face cachée compte.
4. **Branchement** : `Ga3Kit` (`game/scripts/visual/ga3_kit.gd`) derrière
   `BuildingKit.Batch.add` : en bataille (`Ga3Kit.active`, posé par `BattleTerrain.build`), un
   modèle intact du kit dont le type a une variante branchée est remplacé selon la part `share`
   (tirage déterministe sur la position) ; la variante suit l'orientation du cœur et une mise à
   l'échelle `fit` (`footprint` borné à ±15 % d'étirement pour les maisons, `length` ±20 %
   pour l'église, `real` pour le moulin et les accessoires). Instances en `MultiMesh` par tuiles
   de 120 m, trois nœuds LOD par tuile (fin LOD0 90 m, LOD1 280 m × `battle_lod_scale`).
   Palissade du camp retranché : segments générés le long des tronçons (`BattleVillage`).
   `--no-ga3` (après `--`) revient au kit ; rien sous la neige (pas de variante enneigée).
   Aucune règle de jeu ne change : positions, emprises et obstacles viennent du cœur.
5. **Non branchés** : trébuchet et bélier (générés, `wired: false`) — les engins de siège de
   bataille sont animés (`SiegeEnginesFx`), un maillage figé ne les remplace pas.

## Conséquences
- Coût du lot : 1,10 $ (8 objets neufs, maison reprise de S4) ; ≈ 0,05 $ par objet simple,
  0,15 $ avec essai multi-vue, 0,33 $ pour un bâtiment en TRELLIS 2.
- Limites : faces cachées inventées (arrière plus pauvre, pas de porte au dos), éclairage en
  partie cuit dans l'albédo, textures cuites non teintables (pas de variante de saison ni de
  neige), pas de fondations (enfoncement fixe de 0,35 m des bâtiments : sur forte pente un
  côté peut flotter), LOD2 des petites pièces qui perd des longerons (charrette). La
  décimation « collapse » cale parfois (puits LOD2 1 100 au lieu de 450) : sans effet visible.
- Poids : ≈ 22 Mo (glb + textures extraites par l'import Godot) pour 9 objets × 3 LOD.
- Droits : sorties fal.ai utilisables commercialement (conditions fal), modèle TRELLIS
  (Microsoft) sous licence MIT ; images flux-2 et détourage bria produits par l'offre hébergée
  fal (sortie commerciale autorisée). Aucune image tierce en entrée.
- Suite possible : variantes (2-3 maisons, grange), versions enneigées (recoloration du toit
  au shader), fondations ajoutées au nettoyage, trébuchet/bélier figés comme décor de camp.

## Végétation de la carte de campagne (GA3-L2)

Suite de l'ADR 0137 (arbres en cartes et imposteurs, herbe proche). Verdict de la sonde S5 : images
générées pour les arbres (voie A/A'), TRELLIS seulement pour les rochers.

- **Imposteurs générés** : par essence (chêne, hêtre, sapin), une vue de référence `flux-2` puis
  **une seule** planche `nano-banana-2/edit` de 4 × 2 vues tournées de 45° et un détourage `bria`
  de la planche entière (≈ 0,11 $ l'essence). Plus sûr et moins cher que sept retouches séparées
  (0,56 $ l'essence) : les 8 vues d'une même génération partagent éclairage, teinte et échelle.
  Chaque vue est calée dans la boîte de silhouette de la ligne FC2 correspondante (même cadrage
  `ortho` / `foot`, mêmes shaders), couleur ramenée à la moyenne linéaire de cette ligne (teintes
  saisonnières et variété PO3 inchangées), normale approchée (dôme depuis le bord de la silhouette +
  détail de luminance, biais vers le ciel), occlusion depuis la luminance. Même grille 3 × 8 × 256²
  → aucun appel de dessin ni matériau de plus.
- **Les imposteurs remplacent aussi les cartes proches** (`Vegetation.ga3_near_impostors`) : à
  d = 25, triangles des arbres 2,40 M → 0,05 M (test) ; carte entière au Massif central (fenêtre
  1920 × 1080, `--fps-probe`) : 17,0 M → 10,3 M primitives, appels de dessin ≈ 1 010 des deux côtés.
  La caméra proche est rasante (11-30°), favorable à des vues quasi horizontales ; limite connue :
  pas de parallaxe en plongée forte. `--no-ga3-near` garde les cartes FC5 (avec l'atlas de feuilles
  GA3), `--no-ga3-veg` rend tout l'état FC.
- **Herbe** : touffe dense `flux-2` + `bria` (couverture 41 % à 0,5 contre 36 % FC5 et 13 % S5),
  alpha dilaté d'un pixel horizontalement ; la teinte reste celle du shader (luminance seule lue).
- **Rochers** : 3 rochers TRELLIS (dont un repris avec une autre graine : effondrement bloqué à
  1 900 triangles), `ga3_cleanup.py --lod0 120` → 120 / 60 / 18 triangles, albédo 256 relevé
  (`--exposure`) vers la teinte roche du terrain. Semés par `GroundClutter` (roche/lande de la
  splatmap, pente, altitude, terrain de province montagnes / collines ; moins sous forêt), un
  `MultiMesh` enfant par cellule rocheuse (+1 appel de dessin par cellule concernée, sans ombre),
  variante tirée par cellule, niveau de détail commun selon la distance caméra (12 / 24).
- Outils : `tools/blender_scripts/ga3_vegetation_l2.py` (`fal`, `atlas`, `rocks`), pytest
  `tools/tests/test_ga3_vegetation_l2.py` ; test Godot `ga3_l2_vegetation_test.gd`. 0,47 $.

## Figurines de bataille (GA3-L3a, 30/09)

Suite des sondes S2 (TRELLIS multi-vue : faisable, no-go en l'état) et S3 (comparatif Tripo /
Meshy / TRELLIS 2). Unité pilote : le longbowman (`archer_0`), de bout en bout jusqu'en bataille.

- **Consigne du joueur : le moins cher.** Modèle livré : `fal-ai/trellis/multi` (0,02 $) sur les
  vues A-pose **face + dos** ; pas de TRELLIS 2 ni Meshy. Un appel TRELLIS 2 parti avant la
  consigne a servi de comparaison : une fois ramené au budget de triangles par la même chaîne, il
  n'apporte rien de visible à distance de jeu ; TRELLIS vue unique donne un visage plus net mais
  un dos inventé, le multi-vue un dos cohérent (le plus vu en bataille). Coût du lot : 0,48 $
  (dont 0,30 $ de comparaison TRELLIS 2) ; coût d'une unité avec la chaîne livrée ≈ 0,16 $.
- **Référence** : `nano-banana-2/edit` (2K, 0,12 $) depuis la planche SR3 de l'unité : même homme,
  A-pose, mains vides, rien de tenu, trois vues, et **zones teintables peintes en couleurs clés**
  (vêtement de livrée vert saturé, chausses bleu saturé), segmentées par teinte sur l'albédo
  généré (fermeture morphologique : les taches de crasse restent dans la zone). Détourage `bria`
  de la planche, découpe des vues par colonnes d'alpha (`tools/experiments/ga3_fal_figure.py`).
  Le profil généré tient souvent un objet : il est écarté du multi-vue.
- **Maillage** (`tools/blender_scripts/ga3_figures.py`) : les surfaces TRELLIS sont des feuilles
  ouvertes, doublées, non manifold : la chaleur des os échoue sur tous les os et la décimation
  « collapse » cale. D'où : solidify 12 mm, remaillage voxel 8 mm (coque extérieure gardée),
  collapse au plafond des fantassins fins, UV intelligente, **albédo et classes cuits** (Cycles,
  sélection vers active) depuis la source étalonnée (niveaux 0,5 %), poids « heat » sur rig de
  segments dans la pose du maillage puis retour en pose de liaison (chaîne S2), LOD1/LOD2 par
  collapse du LOD0 lié (poids et UV gardés, une texture pour les trois). Plafonds : 11 900 /
  1 350 / 260 triangles, arme comprise ; albédo 1024² (BPTC, ≈ 1,5 Mo PNG).
- **Armes** : procédurales, celles de la figurine fine remplacée (`battle_fine_weapons.longbow` :
  branches sur `Wrist.L`, corde sur `Nock`, flèche sur `Arrow`), UV décalées en u < 0 (non
  texturées), masque « tenu » (lâchées en déroute).
- **Livrée** : codes de matière par face tirés des classes cuites (`C_LIVERY`, `C_CLOTH`,
  `C_PLATE` sur le casque au-dessus de 1,5 m, `C_EXACT` ailleurs) ; alpha de l'albédo = zone
  teintable. Variante `GA3_TEX` du shader skinné : livrée, armoiries de buste et croix du commun
  du jeu (DA1, part de livrée par soldat) ou couleur naturelle par soldat pour les chausses,
  multipliées par la luminance de la texture sur sa moyenne (`ga3_lum`) ; ailleurs l'albédo
  généré. Pas de FG3 (atlas normal/AO) ni d'usure SR2 sur ces figurines : la texture porte sa crasse.
- **Branchement** : manifeste `game/assets/models/battle_ga3/manifest.json` fusionné par
  `BattleSkinned._merge_ga3` par-dessus l'entrée fine (même rig, mêmes clips, même `CAM1`, même
  chargement ; style, noblesse, prises gardés ; `atlas_layer` retiré ; 1 variante). Cadavres,
  imposteurs lointains, figurines de campagne et passants suivent (même matériau).
  `--no-ga3-fig` (après `--`) rend la figurine fine. Aucune règle de jeu ne change.
- **Limites** : une seule variante (plus de visages ni de couvre-chefs alternés : la variété vient
  de la part de livrée et des couleurs de chausses) ; mains en moufle (pas d'os de doigts dans le
  rig exporté, la prise de l'arc reste approchée) ; jupe du jaque liée aux cuisses (léger
  étirement à la marche) ; figurine gonflée de ≈ 6 mm (solidify) ; pas de normal map (le relief
  voxel à 8 mm lisse les piqûres, la texture les porte) ; éclairage de l'image en partie cuit.
- **Extension** : go pour les quatre autres unités à pied (`man_at_arms`, `crossbowman`,
  `sergeant`, `militia`), même chaîne, ≈ 0,16 $ l'unité, une entrée `UNITS` chacune (figure
  remplacée, hauteur du casque, armes procédurales de la recette fine : épée/lance, arbalète et
  pavois dans le dos, guisarme, goedendag) ; pour l'homme d'armes, la classe acier doit couvrir
  tout le harnois (`METAL_Z` par unité). Chevalier monté : ne générer que le **cavalier** (A-pose
  à pied, lié aux os `R:` du rig `cavalry`, la pose assise étant la pose de liaison, alias
  `cavalry_alias`) et garder le cheval fin FG4 avec caparaçon et selle procéduraux ; un cheval
  TRELLIS demanderait un ajustement de squelette quadrupède (pas de repères automatiques) et
  perdrait la robe teintée par cheval.

## Figurines de bataille, extension L3b (30/09)

Quatre unités à pied de plus, même chaîne, une génération `trellis/multi` chacune (0,63 $) :
homme d'armes → `infantry_0`, arbalétrier génois → `archer_2`, sergent → `infantry_1`
(piquiers), milicien → `infantry_5` (goedendag). Une recette par type ; les autres recettes du
même type (`infantry_2/3/4/6/7/8`, `archer_1/4`) restent fines.

- **En données** : `UNITS` de `ga3_figures.py` déclare la figure, la hauteur du casque, les objets
  tenus **par nom de la recette fine** (masque de variante, kwargs et surcharges SR3b lus dans
  `battle_skinned_figures` / `battle_fine_sr`, construits comme `build_figure`), la hauteur de la
  classe acier `metal_z` (0 pour le harnois : l'acier n'est jamais teint) et les clips des rendus.
- **Faces peintes** : les faces `C_ARMS` de l'équipement (écu, pavois) gardent l'UV d'armoiries ;
  `GA3_TEX` ne les remplace pas par l'albédo généré.
- **Îlots** : seuls les îlots voxel < 2 % sont retirés (jambes détachées sous un tabard).
- **Contrôle** : rendus du LOD0 exporté skinné avec la texture d'os du jeu, à côté de la figurine
  fine dans la même pose.
- **Limites ajoutées** : épée au fourreau de la référence fondue au maillage de l'homme d'armes ;
  pas d'armoiries sur le jupon (livrée unie ; l'écu les porte). Points ouverts : variantes de
  visages (une seule figure par recette), mains en moufle, usure SR2 sur les figurines GA3.

## Figurines de bataille, extension L3c : cavalier du chevalier (30/09)

Recette `cavalry_0` (chevaliers : `unit_knights`, chevaliers bretons, teutoniques, serbes) ;
`cavalry_3` (gendarmes, harnois blanc) et `standard_1` (porte-étendard) restent fines. Une
génération `trellis/multi` (0,16 $) depuis `sr3/knight_mounted.png` redessiné **à pied** (A-pose,
jupon vert clé sans meuble, harnois acier, ni épée ni fourreau fondus).

- **Chaîne** : toute la chaîne L3b sur le rig `human` (mêmes données d'os que le cavalier fin :
  proportions fines), puis translation dans le repère du cavalier en selle (`Mount.r_rest`) et
  groupes renommés `R:` ; la pose de liaison du rig `cavalry` est le cavalier **debout sur la
  selle**, les clips le mettent en selle (jambes aux étriers). Plafond `RIDER_CAP`
  (9 000 / 1 000 / 180, équipement compris ; LOD2 à 284 à cause de l'équipement).
- **Cheval** : pas reconstruit ; les triangles du cheval, du harnais, de la selle et du
  caparaçon (sommets sur les seuls os du cheval) sont repris du fichier fin exporté
  `battle_fine/cavalry_0_lod*.mesh.bin` avec leurs UV d'atlas FG3 ; la figurine générée garde
  donc `atlas_layer` (`fine_horse` au manifeste) et le matériau cumule `FG3_BAKED` et `GA3_TEX`
  (robe CC0 en relief, caparaçon cuit, usure SR2, lance CR4). Les sommets du cavalier ont une
  source d'atlas nulle (aucune lecture FG3). Conséquence : reconstruire la figurine fine
  `cavalry_0` impose de relancer `ga3_figures.py -- knight` (pytest de contrôle).
- **Équipement** : écu (armoiries du jeu, `C_ARMS`), lance (`battle_skinned_cavalry.lance`,
  masque de la recette tel quel comme les cavaliers fins), épée au fourreau
  (`battle_fine_equipment.scabbard`, repères lus sur le rig). Acier en `C_PLATE`/`C_EXACT`
  (`metal_z` 0), jamais teint ; jupon en livrée.
- **Limites** : une seule variante (le bassinet ouvert de la génération remplace l'alternance
  bassinet à visière / ouvert) ; jupon sans armoiries (l'écu et le caparaçon les portent).
