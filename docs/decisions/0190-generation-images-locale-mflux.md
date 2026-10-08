# 0190 — Génération d'images locale et gratuite (mflux, Z-Image Turbo)

Date : 2026-10-08.

## Contexte
Les miniatures (encyclopédie, événements, portraits) passent par OpenRouter ou fal.ai, payants
et imputés au plafond de 50 $. Il reste environ 150 miniatures de factions (~12 $). Un essai sur
le Mac de développement (M4 Pro, 48 Go) montre qu'un modèle ouvert donne un style d'enluminure
équivalent, en environ 1 min par image.

## Décision
- Backend local `tools/cent_ans_tools/local_art.py` : appelle `mflux-generate-z-image-turbo`
  (MLX). Il est branché comme un modèle, `local/z-image-turbo`, dans
  `openrouter.request_image` : tout pipeline d'images 2D qui accepte un modèle passe en
  local sans autre changement (coût 0, aucune ligne de budget). Le format vient de
  `image_config.aspect_ratio` ; la première image de référence sert de départ img2img.
  Option `--local` des commandes `cent-ans assets …`.
- Modèle : **Z-Image Turbo** (Tongyi-MAI, licence Apache 2.0, usage commercial libre), copie
  quantifiée 8 bits hors dépôt (`~/models/mflux/z-image-turbo-q8`, 10 Go, chemin modifiable par
  `CENT_ANS_MFLUX_MODEL`). Installation : `uv tool install mflux` puis
  `mflux-save --model z-image-turbo --quantize 8 --path ~/models/mflux/z-image-turbo-q8`.
- Écartés : FLUX.1 dev (licence non commerciale), FLUX.1 schnell (dépôt à accès contrôlé, et
  2,5× plus lourd), Ollama (moins de réglages).
- Graine stable dérivée de l'identifiant de l'entrée : un nouveau lancement redonne la même image.
- Le local est le défaut pour les nouveaux lots d'images 2D ; le payant reste pour les cas où
  le local échoue (texte dans l'image, cohérence d'un personnage sur plusieurs images).

## Conséquences
- Aucune dépense, aucune ligne dans `docs/budget.md` pour ces lots.
- Lots lancés quand la machine est calme (le calcul occupe le GPU environ 1 min par image).
- Les blasons décrits dans le prompt ne sont pas respectés : il faudra une image de référence
  (`--image-path`) ou poser les vrais blasons après coup.
