# 0136 — Figurines et bâtiments semi-réalistes (scans CC0, usure procédurale)

Date : 2026-09-30. Statut : acceptée (autonomie donnée par le joueur le 30/09).
Spec : `docs/superpowers/specs/2026-09-30-sr-semi-realiste-figurines-design.md`.
Complète les ADR 0088 (FG3), 0089 (FG5), 0104 (GA1), 0105 (GA2/GA5).

## Contexte

Le joueur veut des figurines « qui ressemblent à de vrais soldats en armure » et des décors
semi-réalistes, avec un budget d'images IA serré (NB2 ≈ 20 $ au total). Les figurines fines
avaient un rendu « plastique » : matières de détail générées, aucune usure, silhouettes jamais
confrontées à des références réalistes. Les bâtiments (hors monuments) n'avaient aucune usure.

## Décision

1. **Scans CC0 plutôt qu'IA pour les matières** : 8 des 12 couches de détail (laine, lin,
   futaine, gambison, mailles, cuir, plates, bois) viennent de scans PBR ambientCG, avec leurs
   propres normales, rugosités et reliefs, à l'échelle physique (`tile_m` = `GA1_TILE_SIZE`,
   test de cohérence). Peau, cheveux et robes du cheval restent GA1.
2. **Usure procédurale sans texture** (`weathering`, 0,85, `--no-sr2`) : boue montant des pieds,
   crasse des creux, acier gris sourd et rugueux, liseré d'usure sur les arêtes, rouille basse
   des mailles, teintures passées ; effacée entre 60 et 80 m (pas de saut au LOD2).
3. **NB2 seulement comme référence** : 6 planches réalistes (0,42 $, `docs/research/sr3_*`),
   jamais livrées, servent à corriger les recettes Blender (camail sous le chapel, gantelets,
   gants, bourse et dague, bocle, chausses variées) dans le plafond de triangles existant et sur
   les mêmes os. Pas de soldats générés en 3D (TRELLIS) : squelette et animations l'interdisent.
4. **Bâtiments** : include commun `building_aging.gdshaderinc` (boue au pied, coulures, crasse,
   mousse des toits) dans l'atlas des maquettes, les villes et un nouveau `building_pbr.gdshader`
   qui remplace les StandardMaterial3D des bâtiments de bataille à nombre de matériaux égal
   (`aging`, `--no-sr5`).

## Conséquences

- 0,42 $ d'IA ; le reste à 0 $.
- Ouvert : l'éclat des plates reste fort malgré l'albédo et la rugosité (reflets du ciel) ; perf
  A/B à mesurer sur machine calme ; colombage `TimberFrame`, toits bleus du château Kenney, LOD
  grossier des maquettes (voir `docs/wip/sr.md`).
