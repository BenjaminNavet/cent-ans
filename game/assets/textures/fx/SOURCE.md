# Planches d'effets : sources

## flame_flipbook_video.png (lot AS8d, ADR 0189)

Planche de flammes 8x8 images de 64 px (512x512), RVB = chaleur, A = couverture (codage FA2).
Dérivée de **« Fire آتش 01.ogv »** (Mostafameraji, Wikimedia Commons,
https://commons.wikimedia.org/wiki/File:Fire_آتش_01.ogv), licence **CC0 1.0** : aucune
attribution requise, créditée par courtoisie.

Fabrication : `tools/video_mocap/measure_motion_fx.py flame` (vidéo hors dépôt,
`~/dev/cent-ans-mocap-src/video/free/fx/fire_01.ogv`) :

    uv run tools/video_mocap/measure_motion_fx.py flame fire_01.ogv --crop 1000,440,560,640 \
        --start 6 --seconds 3 --atlas flame_flipbook_video.png --columns 8 --rows 8 --frame-px 64

Découpe par couleur (flamme chaude et claire ; feuilles vertes, rochers et visage hors clé),
boucle fermée par fondu des 8 dernières images, pied fondu. Non utilisée par défaut : à choisir
par `fire.flipbook` dans `data/fx/map_fire_wind.json`.

## flame_flipbook.png, smoke_flipbook.png

Voir `data/fx/fire_flipbooks.json` (lot FA2, Unity Labs Paris, CC0).
