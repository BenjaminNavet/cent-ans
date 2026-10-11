# TP — essai de peinture de texture par IA (retexturage)

Demande du joueur (11/10) : essai sur 2-3 assets cassés d'une étape de retexturage locale et gratuite
(rendus de la géométrie → images guidées par la profondeur → projection sur les UV).

## Procédé essayé
1. `tools/experiments/retex.py render` : maillage lod0 normalisé, vues orthographiques (lacets 0/90/180/270,
   élévation 25°), carte de profondeur + normales + rendu de la texture actuelle (rastériseur numpy).
2. `paint` : Z-Image Turbo + ControlNet Union 2.1 (mflux local, `--control depth:…`), prompt du lot DN.
3. `project` : rastérisation en espace UV (position + normale par texel), visibilité par z-buffer, poids
   (n·v)⁴, fusion des vues ; texels non vus = texture d'origine ; dilatation des bords d'îlots UV.
4. Planche avant/après (face, dos) via `tools/blender_scripts/ga3_compare_render.py`.

Sorties hors dépôt : `~/dev/cent-ans-raw/tp/<id>/`.

## État
- [x] script `tools/experiments/retex.py` + lanceur `retex_mflux.py` (profondeur passée telle quelle :
      mflux ré-estimait la profondeur avec DepthPro, +1,9 Go et faux pour une carte déjà exacte)
- [x] mécanique validée sur `city_west` (1 vue) : alignement correct, maçonnerie plus nette
- [x] choix des assets : `dn_back_check` sur 459 modèles buildings/props/siege → 18 `dark_back` ;
      essai sur `castle_empire_rhine` (0,48), `abbey_byz_serbia` (dos 39,8), `fort_postern_gate` (0,56)
- [x] 4 vues × 3 assets (lot `~/dev/cent-ans-raw/tp/run3.sh`, journal `run3.log`)
- [x] planches (6 panneaux : avant / normales seules / après, face et dos) : `~/dev/cent-ans-raw/tp/apercu2.png`

## Verdict provisoire (11/10)
- Corrigés en route : palette de recalage prise sur la vue la plus claire (et non la 1re, qui était le dos
  sombre) ; poids |n·v| car les maillages TRELLIS mélangent les orientations de faces (abbaye : 23 → 67 %
  de texels repeints).
- **Trouvaille** : une part des « dos sombres » vient de faces à normale inversée (abbaye 41 %, poterne
  12 %, château 4 %). Les retourner (vote de visibilité sur les 4 vues, `inverted_faces`) éclaircit déjà
  l'abbaye sans rien repeindre : correctif gratuit, applicable aux 459 modèles.
- `--dark-only` (seuls les texels plus sombres que 0,8 × la bonne face) garde la face d'origine intacte et
  donne un dos cohérent : château bon (toits du dos gris au lieu de rouges), abbaye bonne.
- Défauts : la peinture invente des éléments (poterne : une 2e porte au dos d'un mur plein ; château :
  pseudo-lettres) ; repeinture complète = teinte uniforme beige, perte des couleurs locales.

## Notes
- ControlNet Union 2.1 (6,7 Go, cache HF) + base locale `~/models/mflux/z-image-turbo-q8` via
  `--model <chemin> --base-model z-image-turbo-controlnet-union-2.1` (sinon mflux retélécharge la base).
- 45 s/étape à 1024 px quand une autre session occupe le GPU → vues en 768 px, verrou `dn/gpu.lock`.

## Normales inversées : correctif appliqué (11/10)
`tools/experiments/fix_inverted_faces.py` : vote de visibilité sur 8 vues, retourne l'enroulement des faces
vues de dos (+ normales), n'édite que les tampons d'indices/NORMAL du glb, n'écrit que si la mesure « vue
Godot » (faces arrière éliminées → pixels montrant autre chose que la peau) baisse. Appliqué aux 18
`dark_back` × 3 LOD (journal `~/dev/cent-ans-raw/tp/fix_inverted_18.txt`) : abbey_byz_serbia 33 % → 0 %,
chapel_romanesque lod1 20 % → 1 %, souches 27 % → 3 %… ; château/campanile inchangés (dos sombre = texture).
Modèles hors git (paquet `models-v2`) : **publier un models-v3 = feu vert du joueur**. Idée : l'ajouter à
`dn-ingest` et le passer sur les 459.

## Prochaine étape
Avis du joueur. Pistes : passe « normales » seule sur les 18 `dark_back` (puis les 459) ; prompt du dos
sans la porte/entrée ; banc A/B contre TRELLIS.2 local (trellis-mac installé dans
`~/dev/cent-ans-raw/trellis-mac`, attend l'accès DINOv3).
