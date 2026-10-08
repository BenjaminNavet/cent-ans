# AS8b — allures du cheval depuis des planches libres

Branche `feat/as8b`. ADR 0189. Remplace le keyframé « d'après Muybridge » d'AS3.

## État
- [x] Sources téléchargées hors dépôt (`~/dev/cent-ans-mocap-src/video/free/horses/`, un `.LICENSE.txt` par fichier) : GIF Muybridge (marche, trot, galop), « Horses running in a pasture.webm » (CC BY 3.0, non exploitée : pas de points posés).
- [x] `tools/video_mocap/track_quadruped.py` (fit, track) + `data/horse_keypoints.json` (points posés main : galop 12 images, trot 12 images) + `export_horse_gaits.py`.
- [x] `tools/blender_scripts/data/horse_gaits_free.json` (galop, trot).
- [ ] Branchement Blender (`battle_skinned_gaits.py`), recuit, mesures avant/après.
- [ ] SOURCE.md, CREDITS.md, tests.

## Constats
- Le GIF trot est un cycle de 12 images (une foulée), pas 2 foulées de 6 (premier essai raté, corrigé).
- Marche : planche d'un cheval de trait attelé, pattes trop recouvertes pour poser les sabots ; non refaite (voir rapport).
