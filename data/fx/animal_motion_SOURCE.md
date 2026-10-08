# Provenance — mouvement des bêtes et charrettes (lot AS8c, ADR 0189)

Fichiers : `animal_motion.json` (réglages), `animal_motion_measured.json` (mesures brutes et dérivées).
Les vidéos ne sont pas dans le dépôt (`~/dev/cent-ans-mocap-src/video/free/animals/`, un
`LICENSE.txt` par fichier, licence relue sur la page Commons par l'API le 08/10/2026). Seules les
mesures (nombres) sont versionnées ; elles sont reproductibles par
`uv run tools/video_mocap/as8c_measure.py` (suivi optique OpenCV de points posés à la main sur
des images extraites ; `tools/video_mocap/measure_motion.py`).

| Vidéo (Wikimedia Commons) | Auteur | Licence | Sert à |
|---|---|---|---|
| « Passage d'un troupeau de montbéliardes à Boissia (Jura) en juillet 2018.webm » | Benoît Prieur | CC0 1.0 | cadence et foulée des bovins, courbe de pas, balancement de la tête, broutage |
| « Sheeps near Elbe river.webm » | Tvabutzku1234 | CC0 1.0 | broutage et pas des moutons |
| « Two Horse Drawn Covered Wagons.webm » | Thomas Farley | CC0 1.0 | cahot, roulis, vitesse de la charrette, foulée du cheval de trait |
| « Horse standing MVI 7491.MOV.ogv », « Horse chewing MVI 7493.MOV.ogv », « Horse eating MVI 7496.MOV.ogv » | Rama | CC BY-SA 2.0 fr | mâchonnement, durée et angle de la tête qui descend, queue, immobilité du cheval au piquet |

« Ploughing.ogv » (Sundar, CC BY-SA 4.0) a été téléchargée mais **non utilisée** : 352×288, bœufs dans
la boue, aucun cycle de pas mesurable.

## Licence des fichiers de données
Les courbes de `animal_motion.json` et `animal_motion_measured.json` dérivent en partie de vidéos
**CC BY-SA 2.0 fr** (Rama) : ces deux fichiers de données sont donc diffusés sous
**CC BY-SA 2.0 fr**, avec attribution à Rama, « Horse standing / chewing / eating MVI 7491 / 7493 /
7496 », Wikimedia Commons ; le reste du jeu n'est pas concerné (ADR 0189). Les autres sources sont
CC0 (aucune obligation, mention par courtoisie).

## Hypothèses (pas des mesures)
Échelles : garrot des bovins 1,4 m, rayon de roue de la charrette 0,6 m (208 px/m). Rapport de marche
(part d'appui) des bovins 0,6 pour `swing_m`. Voir `source.assumed_keys` dans `animal_motion.json`.
Limites : durée des clips de 3 à 17 s, suivi du sabot trop instable (le centre de la jambe est suivi),
une seule charrette sur pavés, vitesse des bovins d'un troupeau au pas lent.
