# AS8d — mesures sur vidéos libres (engins, feu, herbe, drapeaux)

Outil : `tools/video_mocap/measure_motion_fx.py` (OpenCV, uv). Vidéos hors dépôt
(`~/dev/cent-ans-mocap-src/video/free/fx/`, un `*.LICENSE.txt` par fichier, licences relues sur
l'API Commons le 08/10/2026). Règles : ADR 0189.

| Vidéo | Licence | Mesure | Appliqué à |
|---|---|---|---|
| Blide31.ogv (Michael Sachse) | CC BY-SA 3.0 | bras de trébuchet à contrepoids articulé, profil, 25 i/s : basculement de 217° (armé) à 74° (fin du dépassement), soit 146°, en 1,44 s ; verticale franchie à 58 % du temps ; vitesse de pointe 290°/s ; la verge stagne près de la verticale puis retombe (rebond) | `trebuchet.swing_curve.lut` (25 points) remplace `u²(2,2−1,2u)` |
| Canon firing mvi 3662.ogv (Rama) | CC BY-SA 2.0 fr | canon à roues tiré à blanc, 30 i/s : éclair 0,133 s, fumée au maximum à 0,83 s, retombée à 20 % en 5,47 s ; recul < 0,01 longueur de fût (canon calé) | `bombard.flash_s` 0,15, `smoke_s` 5,5 (la fumée du jeu durait déjà 5,5 s) ; recul inchangé |
| Flag of Finland.webm (SFootage) | CC BY-SA 4.0 | drapeau rendu sur noir, silhouette suivie : 0,73 Hz, longueur d'onde 0,40 largeur (2-3 ondes) | `maquette_banner.wave_speed` 4,6 (était 5), `wave_length` 0,4 (était 0,9) |
| Fire آتش 01.ogv (Mostafameraji) | CC0 | luminance de la flamme : pics à 1,0 et 1,33 Hz | `fire.flicker_hz` 1,17 (était 8 rad/s = 1,27 Hz) ; planche `flame_flipbook_video.png` en option |
| Calmly waving grass and flowers.ogv (Pixelmaniac pictures) | domaine public | prairie très calme : 0,17-0,25 Hz, centroïde 0,52 Hz, rafales 4,4 × la moyenne | `grass_measured` (mesure seule) |
| Warwick Castle trebuchet - from the bank.webm (mittfh) | CC BY 3.0 | 12 min de treuillage, aucun tir : bras de 100° à 222° en environ 190 s, rapide au début puis lent (suivi bruité) ; la séquence ne contient pas le lancer | mesure seule |

## Limites
- Le lancer vient de Blide31 (4,5 s, 640×480, machine de démonstration plus petite et plus
  légère que celle du jeu) ; Warwick ne contient que le treuillage.
- Blide31 : le bois pâle se perd sur le ciel, les angles des images 32 à 38 ont été relus à la
  main (option `--override` de l'outil) ; le suivi de la verge après l'image 58 n'est pas fiable.
- La bombarde de la vidéo est un canon calé tiré à blanc : pas de donnée de recul.
- Herbe : balancement quasi nul dans la vidéo (0,04 px), fréquences peu discriminantes.
- Drapeau : fréquence et longueur d'onde d'une animation de synthèse, pas d'un vrai tissu ;
  haut et bas de la silhouette donnent 2 et 3 ondes.
- Planche de flammes : 64 px par image, la caméra bouge et la flamme sort du cadre par moments ;
  jugement visuel non fait (aucune capture), donc optionnelle.
- Non repris : `map_banner.gdshader` (constantes dans le shader), `battle_tree_impostor`
  (arbres, pas de l'herbe), rafales de campagne (échelle stylisée).
