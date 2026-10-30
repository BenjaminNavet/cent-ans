# 0848 — Apprentissage : première campagne en chapitres à objectifs d'état

## Contexte
Le guide F8/TUTO (`data/tutorial/steps.json`) est une visite de l'interface : ses étapes demandent d'ouvrir des panneaux,
il n'enseigne aucun arbitrage et Entrée le contourne sans le dire (audit `docs/audits/2026-10-29-experience.md`, C2).
Le joueur a demandé le 30/10 une prise en main plus facile pour les nouveaux joueurs.

## Décision
- Le guide-visite est remplacé par un scénario « Printemps 1337 » de 5 chapitres (royaume, ost, première bataille, siège, monde),
  lancé depuis le menu « Première campagne (apprentissage) ». La partie continue normalement à la fin.
- Les objectifs portent sur l'état de la simulation (`game_api::progress`), jamais sur un panneau ouvert.
- En mode apprentissage, la fin de saison est refusée tant que l'objectif du chapitre n'est pas atteint ; `SkipChapter` le passe explicitement.
- France, Angleterre et Bourgogne ont un but historique et des textes propres ; les 5 autres maisons jouables suivent les mêmes
  chapitres avec des cibles génériques tirées de leur état.
- La couche didacticiel de bataille s'active à la première bataille de campagne.
- Hors scénario : conseils courts selon la situation (Réglages > Tutoriel), F1 et encyclopédie inchangés.

## Conséquences
- Les étapes et conditions « panneau ouvert » de `steps.json` sont retirées ; celles de TUTO (recrutement, siège, ravitaillement) sont reprises.
- Nouveau fichier de données et schéma de chapitres ; nouveau mode de partie.
- Les seed bots vérifient qu'aucune maison ne reste coincée dans un chapitre.
- Plan des lots : `docs/wip/apprentissage.md`.
