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

## État
- [ ] squelette
- [ ] 1 panaches
- [ ] 2 reflets
- [ ] 3 vent
- [ ] import, smoke, captures (≤ 5)

## Prochaine étape
Squelette puis panaches.
