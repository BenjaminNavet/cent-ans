# FC — FPS de la carte et décor semi-réaliste de la campagne

Date : 2026-09-30. Statut : décidé en autonomie (mandat du joueur : semi-réaliste pour la
campagne ; question du joueur sur le chantier FPS carte). Reprend `docs/wip/fps-carte.md`.

## Constat
- Saccades au zoom/déplacement, dues au rendu (8,6 M primitives, ~1 150 appels à d = 30).
  Mesures du 29/09 faites sous charge : non fiables.
- Arbres : oak/beech/fir de ~90 triangles (détail) et ~20 (bas), aucun imposteur sur la
  carte ; pas d'arbre au-delà de d = 700 ; ~324 000 instances dans la scène mesurée.
- Aucune herbe ni broussaille en volume : l'herbe n'est qu'une couche du shader de terrain.
- Ombres des maquettes jusqu'à `model_shadow_distance` 500 quel que soit le préréglage ;
  moulins de `CampaignLife` toujours ombrés.

## Lots
- **FC0 — Base** : `--bench-map --bench-probe` × 3 sur machine calme (aucun agent, aucune
  compilation), Haute auto ; résultats dans `docs/wip/fc.md`.
- **FC1 — Gains sûrs** : `model_shadow_distance` par préréglage (Basse 0, Moyenne 150,
  Haute 250, Ultra 500) ; ombres des moulins coupées au-delà de `veg_shadow_distance` ;
  mesure A/B.
- **FC2 — Arbres imposteurs** : imposteurs (8 vues, atlas albédo + normale) cuits dans
  Blender à partir d'arbres sources plus riches (feuillage en cartes, écorce), 0 $ ; les tuiles
  lointaines (> `detail_distance`) et ForestDetail passent du maillage bas (~20 tri) à
  l'imposteur (2 tri) ; portée étendue (700 → 1000) si le budget le permet ; shader réutilisant
  `battle_tree_impostor.gdshader` + teinte saisonnière de `foliage.gdshaderinc`.
- **FC3 — Herbe et broussailles proches** : touffes en MultiMesh (cartes croisées, sans ombre)
  sur les couches herbe/lande/lisière près de la caméra (d < ~40), densité selon le préréglage,
  fondu, vent léger ; 0 en Basse.
- **FC4 — Contrôle** : banc A/B contre FC0 (objectif : p50 panoramique −20 %, aucune hausse
  d'appels de dessin), 2 captures (d = 30 et d = 150), ADR 0137.

## Hors périmètre / à faire
LOD grossier des maquettes (appels), 2 cascades d'ombre en Haute (compromis de netteté),
Metal System Trace (manuel).
