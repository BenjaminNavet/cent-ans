# ADR 0017 — Préréglages de qualité du rendu, ciels HDRI et étalonnage par données

Date : 2026-09-25. Statut : accepté. Complète l'ADR 0004 (préréglage « qualité » annoncé).

## Contexte

L'audit visuel A1 (lots A1-05, A1-14, A1-13) demande des ciels réels, un étalonnage par météo et par
saison, de la lumière rebondie, de la brume et des feux crédibles. Ces effets coûtent du GPU, alors que
le banc de bataille (40 régiments, ~4 600 soldats) est déjà proche de 60 images/s sur la machine de
développement, et que le réglage par défaut ne doit pas perdre plus de 10 %.

## Décision

- **Quatre niveaux de qualité** (`RenderQuality`, `game/scripts/visual/render_quality.gd`) : Basse,
  Moyenne, Haute (défaut), Ultra ; réglage `video/quality` (Réglages > Affichage), forçable par
  `--quality=` pour les mesures. Un niveau fixe les coûts globaux (atlas d'ombres, filtres doux, SSAO,
  SSIL, grille du brouillard volumétrique, MSAA) et, par environnement enregistré, les effets (SSAO,
  SSIL, SDFGI en Ultra et en bataille seulement, brouillard volumétrique « par mauvais temps » en Haute
  et permanent en Ultra, portée des ombres). Changer le réglage réapplique tout à chaud.
- **Ciels et étalonnage sont des données** (`data/fx/atmosphere.json`, schéma
  `fx_atmosphere.schema.json`) : panoramas HDRI Poly Haven CC0 par météo et par saison (bataille) ou par
  saison (campagne), luminance visée, et étalonnages décrits par des paramètres lisibles (température,
  lift/gamma/gain, contraste, saturation, virage) convertis à l'exécution en LUT 3D
  (`AtmosphereLibrary`). Pas de fichier LUT binaire à maintenir.
- Le ciel HDRI est **tourné pour aligner son soleil sur la lumière directionnelle** et **plafonné en
  luminance** : le soleil peint dans le panorama ne doit pas entrer dans la lumière ambiante (il
  effacerait les ombres portées).
- Feu et fumée en **planches animées procédurales** générées par `tools/cent_ans_tools/fire_flipbooks.py`
  (0 $, reproductible), lues par des shaders de particules dédiés.

## Conséquences

Le rendu reste entièrement côté Godot (aucune règle touchée). Les effets coûteux (SDFGI, brume
volumétrique permanente, SSIL haute qualité) sont réservés à Ultra. Les mesures de performance se font
sous Vulkan (MoltenVK), seul pilote qui rende un temps GPU sur macOS (Metal plafonne les images/s).
