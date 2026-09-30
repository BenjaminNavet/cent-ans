# 0138 — Pipeline image-vers-3D pour le décor de bataille (GA3-L1)

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
