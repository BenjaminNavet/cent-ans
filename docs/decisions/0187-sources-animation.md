# 0187 — Sources d'animation par famille

Date : 2026-10-08. Chantier AS. Détail : `docs/design/2026-10-08-sources-animation.md`.

## Contexte
Le joueur demande s'il existe des modèles gratuits vidéo → animation et si l'on peut animer à
partir de films d'époque, pour toutes les animations du jeu (bataille et campagne). Les modèles
vidéo → 3D les plus forts et tous les générateurs texte → mouvement dépendent de données non
commerciales (SMPL, AMASS, HumanML3D, Human3.6M) ; aucun modèle ni pack cheval gratuit n'est
propre ; RTMW 2D (essai RT) n'améliore pas la main d'arme et ses poids ont une licence douteuse.

## Décision
- Gestes humains : tournages du joueur via MediaPipe (pipeline NT14), Quaternius/Mesh2Motion
  CC0, keyframé en appoint. Triangulation à deux téléphones comme amélioration ouverte.
- Chevaux : keyframé d'après Muybridge (domaine public).
- Animaux et objets de campagne, engins, eau, vent, tissus, feu : procédural (shader, GDScript,
  Jolt pour l'image).
- Exclus : SMPL/AMASS/HumanML3D/Human3.6M et dérivés (dont RTMW3D), RTMW 2D par défaut tant que
  la licence de ses jeux d'entraînement n'est pas établie, DeepLabCut SuperAnimal, fichiers
  Mixamo/Rokoko/MoCap Online dans le dépôt, extraits de films sous droits.

## Conséquences
- Pas de dépense cloud pour l'animation ; fal.ai n'est pas une source d'animation.
- Toute nouvelle source doit montrer la licence de son code, de ses poids et de ses données
  d'entraînement avant d'être essayée par défaut.
- Lots AS1-AS7 proposés dans le document de conception.
