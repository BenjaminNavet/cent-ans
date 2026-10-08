# Sources d'animation — doctrine et lots (chantier AS, 08/10)

Question du joueur : peut-on animer gratuitement, et à partir de films (combats, trébuchet…) ?
Puis : « la question ne se pose pas uniquement au trébuchet ni au mouvement humain, à l'ensemble
des animations que je pourrais avoir dans le jeu, y compris en campagne ».

Entrées : inventaire `docs/research/as-inventaire-animations.md`, sources et licences
`docs/research/as-sources-gratuites.md`, essai RTMW `docs/wip/rt-rtmw.md`, NT12
`docs/research/mocap-gratuite.md`. Décision : ADR 0187.

## 1. Ce que l'enquête établit
- Toute l'animation des personnages passe par une seule chaîne : rig Quaternius → clips Blender
  → texture d'os → MultiMesh. Mocap vidéo (NT13/NT14), CMU (NT12) et CC0 (FA3) s'y greffent.
  Rien ne vient de fal ni de Meshy : la « vidéo → animation » a toujours été locale et gratuite
  (MediaPipe).
- Le reste (engins, eau, vent, tissus, feu, oiseaux, navires, caméras, UI) est procédural
  (shader ou GDScript), sans aucune donnée de mouvement externe.
- Les modèles vidéo → 3D les plus forts (WHAM, GVHMR, TRAM, 4DHumans, MotionBERT, RTMW3D) et
  tous les générateurs texte → mouvement reposent sur des données non commerciales : exclus.
- Chevaux : aucun modèle vidéo ni pack d'animation gratuit et commercialement propre.
- RTMW 2D (essai RT) : bras caché derrière le bouclier 3 à 4 fois plus confiant, mais main
  d'arme 2 à 10 fois plus bruitée que MediaPipe, et poids entraînés sur des jeux dont certains
  sont peut-être réservés à la recherche. Non retenu.
- Films sous droits : reproduire une séquence demande l'accord des ayants droit ; exclus comme
  source. Références propres : nos tournages, domaine public (Muybridge 1878, films anciens),
  reconstitutions et escrime historique sous CC-BY vérifiées vidéo par vidéo.

## 2. Doctrine : une source par famille

| Famille | Source retenue | Pourquoi |
|---|---|---|
| Gestes humains de combat et de rôle | tournages du joueur → MediaPipe (pipeline NT14) ; Quaternius/Mesh2Motion CC0 ; keyframé en appoint | seule mocap propre et sur mesure ; CC0 pour le reste |
| Profondeur des gestes filmés | deux téléphones synchronisés + triangulation (FreeMoCap ou Pose2Sim, outils internes) | corrige la profondeur devinée, défaut n° 1 de NT13/NT14 |
| Chevaux | keyframé d'après Muybridge | aucune source automatique propre |
| Animaux de campagne, charrettes, folk lointain | procédural en shader de sommets (pattes, tête, cahots) sur les maquettes FK | figurines de quelques pixels : un clip serait invisible |
| Foules lointaines | imposteurs existants ; VAT seulement si une famille rigide doit s'animer en masse | coût nul en CPU |
| Engins, murailles, navires | procédural (GDScript + Jolt image) calé sur des mesures réelles | déjà en place (SG, S1/S2, NV) |
| Eau, vent, tissus, feu, météo, oiseaux | shaders et flipbooks CC0 | déjà en place |
| UI, caméras | Tweens et `camera_feel.json` | déjà en place |
| Portraits animés | hors périmètre gratuit (génération d'images payante) | décision séparée du joueur |

Interdits : SMPL/AMASS/HumanML3D/Human3.6M et dérivés ; Mixamo, Rokoko, MoCap Online dans le
dépôt ; extraits de films sous droits ; DeepLabCut SuperAnimal sans accord EPFL ; RTMW3D.

## 3. Lots proposés (0 $)

| Lot | Objet | Méthode | Joueur requis |
|---|---|---|---|
| AS1 | Bêtes de campagne (bœufs, vaches, moutons, cheval de FK) et chevaux du camp de bataille : pas, broutage, queue | shader de sommets sur maquettes rigides, phase par instance | non |
| AS2 | Pieds qui glissent sur la carte : cadence des figurines V2 liée à la vitesse (calage BV2) | GDScript/shader | non |
| AS3 | Cheval : trot, virage, cheval sans cavalier | keyframé Blender d'après Muybridge, recuit des textures d'os | non |
| AS4 | Servants d'engins qui marchent et portent les pierres | clips existants (`carry`, `walk`) + trajets procéduraux dans `siege_crew_fx.gd` | non |
| AS5 | Restes statiques : flammes sur la carte (flipbooks FA2), `maquette_banner` au vent, hampe de carte qui suit la marche, imposteurs d'arbres de bataille au vent | shaders | non |
| AS6 | 3e tournage : coupe horizontale, coup reçu, chute, pas de côté, recul ; à deux téléphones si possible | pipeline NT14 + triangulation | **oui** (filmer) |
| AS7 | Passe de jugement en jeu des lots jugés sur planches (NT7, NT10, NT14, FA3, FK, AN1a) | captures ciblées, une par lot | **oui** (regard) |

Ordre conseillé : AS1, AS2, AS5 (les plus visibles, sans risque) ; AS3, AS4 ; AS6 et AS7 quand le
joueur est disponible.
