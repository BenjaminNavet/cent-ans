# TP — essai de peinture de texture par IA (retexturage)

Demande du joueur (11/10) : essai sur 2-3 assets cassés d'une étape de retexturage locale et gratuite
(rendus de la géométrie → images guidées par la profondeur → projection sur les UV).

## Procédé essayé
1. `tools/experiments/retex.py render` : maillage lod0 normalisé, vues orthographiques (lacets 0/90/180/270,
   élévation 25°), carte de profondeur + normales + rendu de la texture actuelle (rastériseur numpy).
2. `paint` : Z-Image Turbo + ControlNet Union 2.1 (mflux local, `--control depth:…`), prompt du lot DN.
3. `project` : rastérisation en espace UV (position + normale par texel), visibilité par z-buffer, poids
   (n·v)², fusion des vues ; texels non vus = texture d'origine ; dilatation des bords d'îlots UV.
4. Planche avant/après (face, dos) via `tools/blender_scripts/ga3_compare_render.py`.

Sorties hors dépôt : `~/dev/cent-ans-raw/tp/<id>/`.

## État
- [ ] squelette
- [ ] choix des assets (rapport `dn_back_check` sur buildings/props/siege)
- [ ] render / paint / project
- [ ] planches + verdict

## Prochaine étape
Squelette commité → implémenter `render`.
