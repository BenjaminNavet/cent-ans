# Motion capture — état des lieux (09/10, lecture seule)

## 1. État actuel
- Chaîne humaine NT13/NT14 : vidéo du joueur → `tools/video_mocap/extract_pose.py` (MediaPipe 0.10.21 heavy) → `track_disc.py` (disque rouge = bouclier) → `camera_shift.py` → `video_mocap_clean.py` (660 l. : One-Euro, longueurs d'os, appuis, IK, despike poignet) → `nt13_video_trial.py` → `contact_sheet.py`. 25 tests (`test_video_mocap_clean.py`).
- Gestes vidéo : `guard`, `overhead`, `thrust`, `parry` (3 vidéos de 4-6 s), cuits dans `battle_fine/melee/` ; FA3 (Mesh2Motion) prime pour `parry`, `death`, `death_back`. **3 gestes sur ~70 clips viennent d'une vraie captation** ; `slash`, `hit` keyframés.
- Vidéos libres (AS8a) : échec (glissement 0,36-0,47 m), rien promu.
- Muybridge (AS8b) : points posés à la main (`horse_keypoints.json`), Fourier → trot 34 %→5 %, galop 30 %→12 % de glissement. `c_walk` (49 %) et `c_fall` restent keyframés.
- Mesures de courbes (AS8c/d) → `animal_motion_measured.json`, `camp_horse_motion.json`, `trebuchet_swing_curve.json`.
- Règles : ADR 0187, 0189, 0129.

## 2. Forces
- 100 % local, gratuit, licences propres.
- Critères chiffrés et banc comparatif ; « bat l'actuel ou reste en option ».
- Glissement des pieds ramené à 0 sur les gestes NT14.
- Astuce du disque rouge pour le bras de bouclier.
- Coût en jeu négligeable.

## 3. Faiblesses
- Couverture très faible : `slash`, `hit*`, `knockdown`, morts, marche/course/fuite, pas de côté, recul, rotations, et tous les gestes de rôle (arc, arbalète, pique, servants, porteurs, étendard) non captés.
- Tremblement 1,4-1,6° (mains MediaPipe), lame d'estoc 20-40° trop haute.
- Une seule caméra : profondeur devinée, triangulation AS6 absente du code.
- Sources hétérogènes, pas de fondu entre clips d'un cycle.
- Kit grossier sans les clips `melee/`.
- Aucun clip validé en jeu (AS7).
- Sans source : pas du cheval, cheval attelé, servants d'engins, moutons.
- Suivi de sabot optique abandonné ; notes NT13/14 archivées.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Tournage AS6 (prises ci-dessous) → NT14 → promotion par geste | Fort | M | 0 | joueur, animateur |
| 2 | Passe AS7 en jeu (une capture par geste) | Fort | S | 0 | joueur, QA |
| 3 | Pas du cheval : planche Muybridge au pas ou vidéo déjà téléchargée | Moyen-fort | S-M | 0 | animateur |
| 4 | Fondu entre clips de mêlée, vitesses égalisées | Moyen | M | shader | rendu |
| 5 | Gros plan main ou MediaPipe Hand | Moyen | M | 0 | outils |
| 6 | Deux téléphones + triangulation | Moyen | L | 0 | joueur |
| 7 | Recuire `melee/` pour le kit grossier | Moyen | S-M | 0 | tech |
| 8 | Rapport auto de couverture des gestes | Faible | S | 0 | producer |

## Prises à filmer
Caméra posée, profil/¾, corps entier, 60 i/s, disque rouge à gauche, 3 essais de 2-5 s.
1. Coupe horizontale + revers (`slash`).
2. Coup reçu de face, côté, dos (`hit`, `hit_b`, `hit_c`).
3. Renversement, chute arrière (`knockdown`, `death_back`), sur tapis.
4. Mort à genoux, mort en avant.
5. Pas de côté, recul en garde, demi-tour.
6. Marche en garde, charge courte.
7. Fuite, acclamation.
8. Pique à deux mains (manche à balai).
9. Arc (encocher, bander, lâcher), arbalète (manivelle, tir).
10. Servant : porter/déposer une charge, écouvillonner.
11. Porte-étendard : planter, agiter, marcher.
12. Blessé assis/à genoux, ramper.

Les prises 1-5 comblent le trou prioritaire.
