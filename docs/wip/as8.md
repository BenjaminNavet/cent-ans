# AS8 — animations générées depuis les vidéos libres

Demande du joueur (08/10) : « il faut utiliser les videos libres pour generer les animations ».
Règles : ADR 0189 (licences, vidéos hors dépôt), ADR 0187 (sources par famille).
Sources : `docs/research/as-references-video.md`. Vidéos : `~/dev/cent-ans-mocap-src/video/free/<famille>/`
avec un `LICENSE.txt` par vidéo (auteur, URL Commons, licence vérifiée sur la page).

## Lots (worktrees, une branche chacun)
| Lot | Famille | Sources | Sortie | État |
|---|---|---|---|---|
| AS8a | humains (troupes, servants de canon) | Roscheiderhof, joute Eggenburg, Canon firing | clips NT14 cuits, comparés par `-- measure` | lancé |
| AS8b | chevaux (pas, trot, galop, virage) | Muybridge (DP), Horses running in a pasture (CC BY) | angles suivis → `battle_skinned_gaits.py`, recuit | fait sur `feat/as8b` : trot et galop mesurés (trajectoires de sabots), virages héritent du trot ; pas non refait ; détail `docs/wip/as8b.md` |
| AS8c | bêtes et charrettes de campagne | montbéliardes (CC0), Ploughing, wagons (CC0), moutons, Rama | cadences/amplitudes/phases → `animal_motion.json`, charrettes | FAIT sur `feat/as8c` (mesures dans `animal_motion_measured.json`, courbes de pas en table, cahot en 3 raies, roulis ; Ploughing inutilisable ; `as8c_test` vert ; restent : jugement en jeu AS7, foulée des moutons, rapport de marche et levée non mesurés) |
| AS8d | engins, feu, herbe, drapeaux | Warwick trébuchet, Canon firing, Fire 01-10 (CC0), herbe (DP), drapeau | courbes → `siege_engines.json`, `map_fire_wind.json` ; flipbook flammes depuis CC0 | lancé |

## Prochaine étape
Fusion des 4 branches, crédits dans `CREDITS.md`, AS7 (jugement en jeu) étendu aux clips AS8.

## AS8a — humains (branche feat/as8a, 08/10)

État : clips cuits, mesurés, NON promus. Sources vérifiées par l'API Commons (extmetadata) ;
`LICENSE.txt` par vidéo dans `~/dev/cent-ans-mocap-src/video/free/humans/<nom>/`.

| Vidéo | Licence | Verdict |
|---|---|---|
| RoscheiderhofSpaetmittelter2018.webm (Helge Klaus Rieder, 4 min 18, 1080p) | CC BY-SA 3.0 | utilisée : combat d'un homme en brigandine noire à l'écu (33-50 s, caméra à la main) |
| Jousting reenactment Eggenburg.webm (Ekrem Canli, 114 s, 500 Mo) | CC BY-SA 4.0 | inexploitable : cavaliers minuscules et montés, caméra qui bouge ; téléchargement interrompu à 211 Mo (fichier incomplet, à supprimer) |
| Canon firing mvi 3662.ogv (Rama, 17 s, 640x480) | CC BY-SA 2.0 fr | inexploitable : servant caché derrière la roue, petit, puis fumée ; pas de pose extraite |

Le film Roscheiderhof contient aussi un canon (170-215 s) mais les servants y font ~60 px :
non tentés. Aucune marche en armure dans les trois vidéos (pas de `vf_march`) ; pas de
chargement de canon exploitable (pas de `vf_gun_load`).

Pipeline : `ffmpeg` (33-50 s, 25 i/s) → `extract_pose.py` (409/425 images détectées) →
`tools/video_mocap/camera_shift.py` (nouveau : translation du fond par flux LK, retranchée des
repères image pour les contacts de pieds, fichier `<video>_cam.npz` lu par `Clip`) →
`nt13_video_trial.py` (`CLIPS_AS8A`, `-- bake-vf`, `-- measure` ajoute les `vf_` à la ligne du
rôle). Bake : les clips `vf_` ne sont PLUS versionnés (inutilisés, moins bons, dérivés CC BY-SA) : on les recuit hors dépôt avec `nt13_video_trial.py -- bake-vf` (vidéos locales), le dossier `battle_fine/vf/` est ignoré du dépôt.

Clips : `vf_thrust` (i. 104-150), `vf_guard` (148-172, bouclé), `vf_strike` (358-392 ; rôle
`overhead`). Lacet fixé à la main (le lacet auto donnait -32° / 138° : visage et épée peu fiables).

Mesures (`-- measure`, avec camera_shift ; glissement m / tremblement / accél.) :
| rôle | actuel (défaut) | vf_ |
|---|---|---|
| thrust | NT14 : 0 / 1,63 / 18,5 | vf_thrust : 0,433 / 1,62 / 11,9 |
| guard | NT14 : 0 / 0,03 / 0,06 | vf_guard : 0,360 / 0,38 / 4,95 |
| overhead | NT14 : 0 / 1,41 / 15,6 | vf_strike : 0,473 / 1,66 / 12,7 |
Sans camera_shift : glissement 0,35 / 0,37 / 0,49. Cause : MediaPipe suit mal talons et orteils
(armure, jambes 50-80 % visibles) → 8-23 images d'appui seulement, un pied jamais posé sur la
garde ; tremblement de pied 3-4°. Aucun clip n'est meilleur : rien de promu, `melee/` inchangé,
option de jeu non créée (aucune option `data/fx` existante pour ces clips).

Prochaine étape possible : un tournage du joueur (caméra fixe, plein pied) ; ou suivi des pieds
par un second modèle. (`as8a_test.gd` supprimé avec les clips.)
