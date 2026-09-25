# L3 — Villes emblématiques : matériaux et sièges

Branche `worktree-agent-af2974d71a33b3dc3`. Suite de L1 (`docs/wip/l1-paris.md`) et L2
(`docs/wip/l2-villes.md`), ADR 0015. Captures : `docs/audit/captures/l3/`.

## Plan
1. Matériaux : atlas partagé `game/assets/textures/landmarks/` (Texture2DArray albédo + normales,
   Poly Haven CC0 + couches procédurales), indice de matériau dans l'alpha de la couleur de sommet
   (`landmark_city.py`), triplanaire + salissures/coulures/mousse + variation par bâtiment dans
   `landmark.gdshader` (aucune modification nécessaire des scripts qui posent le shader).
2. Toiles de fond de siège pour les 6 villes de L2 (bloc `siege`, `<id>_siege.glb`).
3. Siège dans le plan : `SiegeLayout` (enceinte, portes, rues) lu de `data/landmarks/<id>.json` par
   le cœur, `SiegeWorks::from_layout` dans `sim-battle`, ADR 0026.

## État
- [ ] Matériaux
- [ ] Toiles de fond
- [ ] Siège dans le plan

## Prochaine étape
Télécharger les textures Poly Haven, écrire `game/assets/textures/landmarks/build_textures.py`.
