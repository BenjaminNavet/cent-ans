# RV-F — vie visible en vue régionale (chantier RV, `docs/wip/rv-relief-vivant.md`)

Branche `feat/rv-f`, worktree `gp-rv-f`. Objectif : carte vivante caméra haute (palier moyen,
d ≈ 150-1200), sans pictogrammes (ADR 0124), sans coût FPS notable, rien qui scintille au dézoom.

## Plan (par rendement)
1. Panaches régionaux (`LifeEffects`, un MultiMesh de plus) : fumées fines de taille écran quasi
   constante au palier moyen, couchées par le vent météo, plus nombreuses l'hiver ; relais des
   cheminées 1:1 de près ; effacées sous le parchemin.
2. Reflets du soleil sur la mer (`water.gdshader`) et les fleuves (`river_water.gdshader`) :
   include `water_glint.gdshaderinc`, cellules à l'échelle du pixel (filtrage par footprint).
3. Vent dans les forêts : `forest_wind` (campaign_life) appelée par une ligne `// RV-F` du
   terrain ; rafales de luminosité qui traversent le couvert ; impostors (partie vent).

## État (fini, à intégrer dans feat/rv)
- [x] Panaches régionaux : `life_plume.gdshader` + `LifeEffects._fill_plumes/_update_plumes` ; 2361
  instances (1 appel de rendu), densité saison (`PLUME_DENSITY_PER_BOOST`), incendies sombres
  (siège, dévastation ≥ 45, hameaux brûlés 1/6), palier moyen seulement, coupés par `--life-off=smoke`.
  Piège corrigé : `PROJECTION_MATRIX[1][1]` est négatif sous Vulkan (abs).
- [x] Paillettes de mer : `water_glint.gdshaderinc` (cellules de 4 px, deux grilles fondues selon
  l'empreinte, nappes), dans `water.gdshader` light(). Fleuves non traités (pas de light()).
- [x] Vent : `campaign_wind.gdshaderinc` partagé ; `forest_wind` + 1 ligne `// RV-F` du terrain ;
  imposteurs (flexion, éclaircissement). Non jugé visuellement (animé).
- [x] Smoke vert ; planches `docs/audit/captures/rv-f/` (5 captures).
- Bench (`rv_life_shot.gd --bench`) : bruit ± 2 ms (machine chargée, autres agents) ; pas de coût
  mesurable à d=300/420.

## Points ouverts
- Réglage final des panaches (dernière valeur, entre « trop de traits » et « invisible ») non revu
  en capture : à juger en jeu (`screen_size`, `smoke_alpha`, `PLUME_DENSITY_PER_BOOST`).
- Paillettes : semis de points un peu régulier ; soleil derrière la caméra → surtout la traîne.
- Mouchetures jaunes sur les forêts à d=300 : préexistantes (A/B), pas RV-F.
- Fumées FA (ADR 0164, flipbooks) non réutilisées : trop fines pour la vue régionale.
