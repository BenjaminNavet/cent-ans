# VH5 — Paris vers 1340 à l'échelle 1:1 (format landmark v2, ADR 0078)

Branche `feat/vh5-paris` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`). Référence :
`docs/landmarks-v2.md` (section « Paris et ALPAGE »), addendum VH5 de l'ADR 0078. Liens
symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal ; dylib copiée
de `game/bin/` du dépôt principal. Aucun changement Rust.

## Sources : ALPAGE est téléchargeable (ODbL)
- « Paris en 1380 » (P. Rouet) : voies, îlots, usages du sol, hydrographie ; « Vasserot v1 »
  (A.-L. Bethe) : parcelles 1810-1836. GeoPackage EPSG:2154, cache partagé
  `tools/geo/raw/alpage/` (téléchargé au besoin par `geo/alpage.py`).
- `tools/cent_ans_tools/geo/alpage.py` : lecture GPKG, rues de 1380 (`origin: "alpage"`),
  parcelles Vasserot filtrées (`parcels` = [dE, dN, angle, façade, profondeur]).
- `tools/geo/paris_v2_author.py` : écriture initiale de `paris.json` (identifiants ALPAGE des
  murs, portes, monuments, îlots, Seine + faits datés). Relancer ÉCRASE le fichier ; ensuite
  `cent-ans geo landmarks --city paris` régénère rues et parcelles.

## Moteur commun (rétrocompatible ; Rouen : mêmes 5 838 maisons)
- `landmark_plan.gd` : index en grille des quartiers et des eaux ; eaux en polygone
  (`polygon` + `holes`) ; `_imported_parcels` avant les lanières ; plusieurs ponts
  (`plan.bridges`, `plan.bridge` = premier) avec tablier et maisons recalculés au `reground` ;
  minutage par étape `stats.marks_usec`.
- `town_builder.gd` : construit tous les ponts de `plan.bridges`.
- Schéma v2 : `origin` += `alpage` (rues, eaux) ; `waters.polygon/holes` ; `bridges.certainty` ;
  recette `alpage` ; `parcels` en tableaux de 5 nombres.
- Outil `geo landmarks` : recette `alpage` ; `fine_rivers: []` retire le fleuve fin.

## État : terminé (à fusionner par l'orchestrateur)
- [x] `paris.json` : enceinte de Philippe Auguste (2 rives, 20 portes et poternes, ≈ 5 km),
      Charles V datée (levée 1356-1364, maçonnée 1365+, 6 portes), 119 monuments datés
      (110 présents en 1340), 37 quartiers, 32 espaces libres, 6 ponts (4 en 1340), 5 quais,
      Seine de 1380 en 4 polygones ; 1 227 rues ALPAGE ; 4 772 parcelles Vasserot
- [x] Plan : ≈ 8 770 maisons, 110 monuments, 78 tours ; 2,0 s (machine calme) à 4,9 s (chargée)
- [x] Captures `docs/img/vh5/` (stratégique, transition, vallée, site, Cité, Notre-Dame, Louvre,
      Grand-Pont, rive gauche, Ville)
- [x] i/s (charge ≈ 110-150) : Paris 34-60 i/s, Rouen 49-60 i/s dans les mêmes sessions
      (mesures très bruitées ; une session : 60/60 partout)
- [x] Tests : `vh4_landmarks_test` (Paris ajouté), `zg4_camera_test`, `zg6_towns_test`, `smoke`
      OK ; pytest complet 733 OK ; ruff propre
- [x] Docs : `docs/landmarks-v2.md`, `CREDITS.md` (ALPAGE), addendum ADR 0078

## Limites / suites
- Rendu du fleuve (hors lot, SZ2b) : la Seine fine est un axe unique de ≈ 130 m qui passe sur
  le nord de la Cité et ignore le petit bras ; le lit creusé ne suit donc pas les quais de 1380.
  La ville utilise le lit ALPAGE pour interdire les maisons.
- Exagération du relief (S1) : montagne Sainte-Geneviève et Montmartre en pics.
- Parcellaire Vasserot : ≈ 70 % des parcelles de la ville close gardées ; cœurs d'îlots vides
  (cours et jardins), lanières générées pour le reste. Faubourgs : lanières générées seulement.
- Monuments : 110 maillages séparés (une instance chacun) ; i/s Paris parfois sous Rouen :
  fusion par cellule à envisager (VH8).
- Bièvre non tracée ; moulins des ponts (`mills_at`) non rendus (comme VH4) ; ponts en
  maçonnerie même s'ils sont en bois.
- Temps de plan dominé par les lanières générées (≈ 1,2-2,4 s) : optimisable.

## Faits à faire relire par l'historien
Grand-Pont en 1340 (bois ? largeur 10 m restituée ; les 106 × 27 m sont ceux de 1413) ; pont
aux Meuniers et Planches de Mibray en 1340 ; hauteur de la flèche de Notre-Dame (≈ 78 m restitués)
et état des travaux vers 1340-1345 ; Charles V : levée de terre 1356-1364 puis maçonnerie datée
1365 ; Louvre de Charles V dès 1364 ; donjon du Temple (avant 1310) ; fondation de Saint-Victor
(1113, tradition) ; Bernardins (église de 1338 inachevée) ; noms ALPAGE des portes (porte aux
Aveugles = porte Saint-Honoré de Philippe Auguste ? porte du Temple = porte de Braque ? porte
Baudoyer/Saint-Antoine) ; fossés de Philippe Auguste absents en 1340 ; densités et faubourgs
tirés des zones bâties de 1380 (postérieures de 40 ans).
