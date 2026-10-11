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
- [x] script `tools/experiments/retex.py` + lanceur `retex_mflux.py` (profondeur passée telle quelle :
      mflux ré-estimait la profondeur avec DepthPro, +1,9 Go et faux pour une carte déjà exacte)
- [x] mécanique validée sur `city_west` (1 vue) : alignement correct, maçonnerie plus nette
- [x] choix des assets : `dn_back_check` sur 459 modèles buildings/props/siege → 18 `dark_back` ;
      essai sur `castle_empire_rhine` (0,48), `abbey_byz_serbia` (dos 39,8), `fort_postern_gate` (0,56)
- [ ] 4 vues × 3 assets (lot `~/dev/cent-ans-raw/tp/run3.sh`, journal `run3.log`)
- [ ] planches + verdict

## Notes
- ControlNet Union 2.1 (6,7 Go, cache HF) + base locale `~/models/mflux/z-image-turbo-q8` via
  `--model <chemin> --base-model z-image-turbo-controlnet-union-2.1` (sinon mflux retélécharge la base).
- 45 s/étape à 1024 px quand une autre session occupe le GPU → vues en 768 px, verrou `dn/gpu.lock`.

## Prochaine étape
Attendre `run3.log` (ALLDONE), lire les `sheet.png`, verdict.
