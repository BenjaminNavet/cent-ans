# 0354 — Convois vivants : taille monde fixe, aucun pop, modèles Meshy

Date : 2026-10-14. Statut : accepté. Révise ADR 0340 (VG2 : taille croissante avec la distance).

## Contexte
Le joueur juge les caravanes, charrettes et bêtes de la carte peu crédibles : modèles TRELLIS 1 ou
boîtes procédurales, apparitions/disparitions brutales, taille qui grossit avec le dézoom.

## Décision
- Taille monde fixe pour les figurines (`FolkPool`) et les accessoires routiers du calque campagne
  (`CountrysideLayer`) : plus de croissance avec la distance ; échelle maquette nettement plus grande.
- Plus aucun pop : semis ancré sur le monde (indépendant du point visé), éclaircie au dézoom par un
  aléa fixe d'instance comparé à un seuil continu en shader (fondu tramé), fondus au bord de la zone
  et à la portée au lieu d'un vidage instantané.
- Modèles : charrettes, caravanes, bouviers et bêtes régénérés en Meshy 6-lite (chaîne MX) ; les
  accessoires FK2 procéduraux sont remplacés par ces glb DN.

## Conséquences
- Densité à l'écran plus faible en vue lointaine (assumé : réaliste).
- Détails d'implémentation et réglages : `docs/wip/cv-convois-vivants.md`.

## Détails d'implémentation CV-B (14/10)

Taille fixe
- `FolkPool.world_scale()` = `folk_scale` / `meters_per_px`, sans croissance : `folk_scale` 300 → 600
  (`town_maquettes.json` § props). Un homme de 1,8 m fait ≈ 1,5 unité, une charrette de 3,5 m ≈ 2,9
  unités (cohérent avec un arbre généralisé). `folk_screen_from` / `folk_screen_exponent` et l'écart
  `spread` (puissances de deux) sont supprimés (code, JSON, schéma).
- `CountrysideLayer` : les modèles marqués `"fixed_size": true` dans `map_countryside.json` (12 :
  charrettes, caravanes, bouviers, cabane roulante) prennent en maquette le même grossissement
  (`folk_scale` × `size_scale`), sans `size_k`, `max_mult` ni dépendance à la distance ; leur écartement
  suit (spread = mult). Hors maquette, l'ancien comportement reste.

Aucun pop (FolkPool)
- Aléa d'instance h ∈ [0, 1) : les fournisseurs ouvrent une unité (`begin_unit(h)` / `end_unit`) avec un
  h tiré de graines du monde ; tout ce qui la compose (charrette + charretier + marchand + gardes,
  attelage de labour, troupeau et berger, scène) partage h. Sans unité, h dérive de la position de
  départ. h est codé dans l'échelle de l'instance (`× (1 + 0,004 h)`, taille fixe à 0,4 % près) et lu
  en shader (`folk_fade.gdshaderinc`) ; il remplace `INSTANCE_ID` pour le choix de variante / clip des
  figurines (variante `FK_TRAVEL`), donc un replacement ne change aucune figurine.
- Semis ancré : routes = h par tronçon (`FolkRoutine.segment_h`) ; champs, pâtures, forêts = grille
  emboîtée (`cell_h` : les cases de niveau l, multiples de 2^l, ont h dans [4^-(l+1), 4^-l[ ; on ne
  parcourt que le réseau de pas 2^floor(log4(1/seuil)), coût ∝ seuil) ; marchands = h par trajet et
  charrette ; scènes = h 0 (jamais éclaircies). Plus aucune dépendance au rayon ni au point visé
  (le rayon ne fait que borner la zone parcourue ; plafond dur rarement atteint).
- Éclaircie : `keep_for(d) = clamp((folk_keep_from / d)^folk_keep_exponent, folk_keep_min, 1)` ×
  facteur de budget d'image (fondu 1 → 0,5 à 0,6/s, plus de replacement au passage budget). Shader :
  visible si h < keep / (1 − bande), fondu tramé (Bayer 4×4, `discard`, pas de transparence).
  Le CPU ne pose que h < `ceiling` = seuil × 1,5 + 0,03 : un zoom avant de 50 % n'a pas à replacer.
- Fondus : uniform `fk_zone` (point visé courant, rayon intérieur `folk_edge_inner` × rayon de vue,
  extérieur = rayon de vue) mis à jour chaque image ; uniform `fk_thin.z` = fondu de portée de
  `folk_range_fade` × `folk_range` à `folk_range`, après quoi `clear()` ne se voit plus.
- Zone placée = rayon de vue × `folk_zone_margin` (1,4). Replacement quand la zone de vue entame
  75 % de la marge (différé jusqu'à ce que la caméra se pose, 0,5 s au plus), 93 % : immédiat ; aussi
  quand le seuil d'aléa posé ne couvre plus l'éclaircie (zoom avant) ou que la zone placée est
  devenue 2,8× trop large. Un replacement ne change rien à l'écran (mêmes instances).
- `pool_cap` (map_scenes.json) passe de 260 à 700 : plafond dur de garde-fou, plus la densité visible.

CountrysideLayer : fondu tramé (shader, uniform `dither`) des modèles `fixed_size` pour la portée, la
saison, l'amincissement par rang et les cellules neuves (`cell_grow`) au lieu d'un rétrécissement.
Les cellules gardaient déjà un chargement avec hystérésis et un retrait progressif (ZF-B).

### Attelage et mouvement (intégration, 14/10)
Les glb DN de charrette n'incluent pas de bête : `FolkModels.PROPS` porte un champ `team`
(charrette marchande → 2 chevaux, charrette paysanne → 1 bœuf, charrette de pierre et charrue →
2 bœufs) et `FolkPool.add_team` pose les bêtes devant le timon, à la même vitesse et avec le même aléa
d'unité. Le mouvement procédural (`animal_motion.json`) est recalé sur la géométrie mesurée des glb DN ;
le shader accepte deux essieux (`am_axle`).
