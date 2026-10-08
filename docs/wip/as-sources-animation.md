# AS — sources d'animation (tout le jeu)

Chantier de réflexion ouvert le 08/10 à la demande du joueur : pour chaque famille animée
(humains, chevaux, engins, navires, campagne : armées, chariots, scènes de vie, eau, végétation,
UI…), choisir la meilleure source gratuite et compatible usage commercial : vidéo → mouvement,
bibliothèques mocap, procédural/physique, keyframé.

## État (08/10)
- FAIT : inventaire (`docs/research/as-inventaire-animations.md`), sources et licences
  (`docs/research/as-sources-gratuites.md`), essai RTMW non retenu (`docs/wip/rt-rtmw.md`),
  doctrine `docs/design/2026-10-08-sources-animation.md`, ADR 0187.
- Contraintes : rien qui dépende de SMPL/AMASS/HumanML3D/H36M ; vidéos personnelles hors dépôt
  (`~/dev/cent-ans-mocap-src/`) ; pas d'extraits de films sous droits ; batailles navales sans 3D.

## Prochaine étape
08/10 : joueur d'accord (« ok »). Vague AS1-AS5 lancée en parallèle, un worktree par lot
(`../gp-as1` … `../gp-as5`, branches `feat/as1` … `feat/as5`, notes `docs/wip/as1.md` …),
ADR réservés 0188-0192. Ensuite : relecture, fusion ff dans main, une capture de contrôle
par lot (session principale), suppression des worktrees. AS6 (tournage) et AS7 (jugement en
jeu) attendent le joueur.
