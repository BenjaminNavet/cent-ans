# 0189 — Animations tirées de vidéos libres

Date : 2026-10-08. Chantier AS (lot AS8). Complète l'ADR 0187.

## Contexte
Le joueur demande d'utiliser les vidéos libres repérées (`docs/research/as-references-video.md`)
pour générer les animations, pas seulement pour les caler. Les vidéos sont en domaine public,
CC0, CC BY ou CC BY-SA ; aucun modèle de pose de quadrupède n'est utilisable commercialement
(ADR 0187).

## Décision
- Vidéos téléchargées hors dépôt (`~/dev/cent-ans-mocap-src/video/free/`), jamais commitées ;
  seuls les clips cuits, les courbes mesurées et les scripts sont versionnés.
- Ordre de préférence : domaine public et CC0, puis CC BY, puis CC BY-SA.
- Un clip ou une courbe dérivé d'une vidéo CC BY ou CC BY-SA est traité comme une adaptation :
  auteur, titre, URL et licence dans le `SOURCE.md` du dossier et dans `CREDITS.md` ; un fichier
  dérivé de CC BY-SA porte la même licence (fichier de données seul, le reste du jeu n'est pas
  concerné).
- Humains : pipeline MediaPipe existant (NT13/NT14). Quadrupèdes : points clés posés sur
  quelques images puis suivis automatiquement (OpenCV, ou TAPIR si licence des poids Apache
  vérifiée), en vue de profil, angles d'articulation dans le plan sagittal ; Muybridge en
  domaine public pour les allures. Engins, charrettes, tissus, végétation, feu : courbes et
  fréquences mesurées par suivi d'image.
- Un clip tiré d'une vidéo ne remplace le clip actuel par défaut que s'il est meilleur sur les
  mesures (glissement, tremblement, appuis) ; sinon il reste derrière une option.

## Conséquences
- Pas de dépense cloud. Les crédits d'auteurs s'allongent.
- Les familles sans vidéo libre (bombarde, mangonneau, porteurs de pierres, cheval attaché)
  restent procédurales ou attendent un tournage.
