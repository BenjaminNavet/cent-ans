# AS8c — bêtes et charrettes mesurées sur vidéos libres

Branche `feat/as8c` (partie de `main` d83187ee5). ADR 0189, plan `docs/wip/as8.md`.

## État
- Vidéos téléchargées hors dépôt : `~/dev/cent-ans-mocap-src/video/free/animals/<slug>/` (montbéliardes CC0,
  Ploughing CC BY-SA 4.0, wagons CC0, moutons de l'Elbe CC0, Rama 7491/7493/7496 CC BY-SA 2.0 fr),
  un `LICENSE.txt` par fichier (licence lue par l'API Commons le 08/10).
- `tools/video_mocap/measure_motion.py` : `sheet` (planche à grille), `track` (LK + MIL), `overlay`, `analyse`
  (fréquence, harmoniques, profil replié sur 16 phases).

## Prochaine étape
Mesures par clip → `data/fx/animal_motion_measured.json`, puis shaders (courbes en LUT), schéma, `as8c_test.gd`,
`SOURCE.md` + `CREDITS.md`.

## Points ouverts
- Le suivi des sabots par LK/MIL glisse sur le sol : relevés de sabots à la main sur quelques images si besoin.
