# PF1 — performances (préréglages, zoom comté, fuite particules, user://)

## Tâches
1. [ ] Préréglages Basse / Moyenne efficaces (relief, LOD bataille, végétation, particules, ombres, MSAA, SSAO)
2. [ ] Zoom comté en Haute ≤ 16,7 ms (LOD / culling du relief fin, MSAA gardé)
3. [ ] Fuite « ParticlesShaderRD were never freed » à la fermeture du jeu exporté
4. [ ] user:// du jeu exporté isolé + ADR

## État
Fusion RL1 + main faite, build debug + import OK. Mesures de référence en cours.

## Prochaine étape
Mesurer la base (`--map-ab=150`, primitives par couche), puis LOD du relief fin.
