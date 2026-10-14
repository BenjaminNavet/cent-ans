# 0356 — Villes emblématiques : maquette générée à la place de la maquette Blender drapée

Date : 2026-10-14. Chantier VM (`docs/wip/vm-villes-maquettes.md`).

## Contexte
Sur la carte de campagne, les villes emblématiques (`data/landmarks/`) sont des maquettes Blender tirées du
plan historique (`LandmarkModel`, drapées sur le relief par `landmark.gdshader`, ADR 0078, 0158). Les autres
lieux affichent des glb générés (ADR 0214). Le joueur (14/10) a comparé un Paris généré en style maquette
(Z-Image puis Meshy) et demande de le substituer, puis de faire de même pour les autres villes 1:1.

## Décision
1. `data/art/dn_campaign_models.json` gagne une section `landmarks` : id de ville → glb ingéré (`path`,
   `lod`, `width_factor`, `yaw_deg`, `albedo_gain`).
2. `LandmarkModel.create` prend ce glb s'il est installé (sinon repli sur la maquette Blender). Pas de drapé :
   le glb a son propre socle ; largeur = diamètre du cœur (`scale.core_radius_px` × `landmark_scale`) ×
   `width_factor` ; posé à la hauteur moyenne du relief affiché sous le cœur, reposé quand elle change.
   Zone, cœur, absorption des lieux voisins, étiquettes : inchangés.
3. Nouvelle classe d'ingestion `landmark_city` (40 000 / 12 000 / 4 000 triangles, texture 2048, socle gardé) :
   une seule instance par ville, LOD0 affiché.
4. Les batailles de siège et les plans v2 (`LandmarkCityLayer`) gardent les plans Blender.

## Conséquences
- Les villes emblématiques se lisent comme des maquettes, cohérentes avec les autres lieux ; on perd la
  fidélité du plan (tracé exact de la Seine, éléments datés `set_year`).
- Le glb voyage dans le paquet de modèles (ADR 0212) ; sans paquet, la maquette Blender reste.
- Coût : 15 crédits Meshy par ville.

## Addendum (14/10) — Le Caire, Gizeh, Pétra
- Une ville emblématique peut n'avoir que l'ancre et l'échelle (`data/landmarks/cairo.json`, `monuments` vide) :
  sans glb généré installé, elle retombe sur la maquette générique du lieu.
- Les merveilles hors des villes (pyramides de Gizeh et Sphinx, Pétra) sont des sites isolés du décor
  (`map_landmarks_extra.json`), modèle à leur taille réelle, grossi à l'écran par `fraction`. Le drapeau
  de type `known` les montre même sous le brouillard : ce sont des lieux connus de tous en 1337.
