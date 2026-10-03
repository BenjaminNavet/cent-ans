# 0168 — Relief de campagne vivant (fin de l'estompage)

Statut : accepté (chantier RV, 2026-10-03, docs/wip/rv-relief-vivant.md)

## Contexte
Le joueur voyait encore « la carte IGN d'Europe avec ses petites ombres de relief », un effet
« carte papier ». Mesure (RV-A) : HB6 (ADR 0143) avait mis un gain local négatif sur les collines
(`gain_far` −0,4) ; le maillage montrait les collines à ≈ ×2,6 mais les normales d'ombrage
(`shading_relief` 2,1) les poussaient à ≈ ×5,4. Ombrage sans volume, à haute fréquence : un
estompage. Le soleil, lui, était déjà rasant (18-21°, ADR 0156) ; le 45° du .tscn n'est qu'un
défaut écrasé au chargement. Les ombres de nuages CV1 existaient mais valaient 0 par temps clair
(TB2) : carte figée.

## Décision
- **Volume plutôt que normales** (RV-A) : `shading_relief` 2,1 → 1,15 (régional),
  `shading_relief_near` 1,3 → 1,0 ; pente d'ombrage = basse fréquence (heightmap ×4 plus large)
  + pente fine atténuée à 35 % en vue régionale, pleine de près ; `gain_far` −0,4 → 0. La roche
  et les falaises gardent la pente brute.
- **Lumière froide/chaude** (RV-B) : ambiance ciel froide, exposition 0,98, petite variation
  déterministe du soleil par tour (±3° d'élévation, ±15° d'azimut, jamais à l'est), perspective
  aérienne bleutée selon l'inclinaison caméra. Réglages `data/fx/campaign_lighting.json` (schéma).
  Le soleil reste piloté par `CampaignAtmosphere` (ADR 0156 inchangé).
- **Occlusion de vallée précalculée** (RV-D) : `cent-ans geo relief-occlusion` (horizon 16
  directions, 6/12/25 km) → `data/map/relief_occlusion.png` (L8, 3,4 Mo) ; vallées plus sombres et
  froides, crêtes éclaircies, sortie AO modulée ; repli propre si le fichier manque.
- **Ombres de cumulus** (RV-C) : champ partagé avec les nuées visibles, décalé selon le soleil,
  dérive au vent météo, ~22 km, force 0,40 ; sol, mer (lumière directe) et imposteurs d'arbres.
  La règle TB2 « pas d'ombre de nuage par temps clair » est levée pour les cumulus.
- **Vie régionale** (RV-F) : panaches de fumée par colonie (MultiMesh, sombres sur villes
  assiégées / provinces dévastées), paillettes du soleil sur la mer (dans le chemin du soleil
  seulement, `glint_tail` 0,03), rafales de vent sur forêts et imposteurs.

## Conséquences
- Relief plus doux en vue régionale ; s'il manque de lecture, remonter `shading_relief` vers 1,3
  ou `relief_fine_far` vers 0,5, sans revenir à 2,1.
- Coût : +4 lectures de heightmap par fragment au-delà d'une empreinte 0,3, 1 draw call
  (≈ 1000 panaches visibles), bruits sur eau/forêt ; non mesuré sur machine calme.
- Ouvert : arbres proches (FC6) sans ombre de nuage ni occlusion ; fleuves sans paillettes ;
  réglage des panaches à juger en jeu (`screen_size`, `smoke_alpha`, `PLUME_DENSITY_PER_BOOST`).
