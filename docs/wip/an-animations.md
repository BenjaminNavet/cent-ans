# AN — stratégie d'animation (tout le jeu)

Chantier de réflexion ouvert le 08/10 à la demande du joueur : pour chaque famille animée
(humains, chevaux, engins, navires, campagne : armées, chariots, scènes de vie, eau, végétation,
UI…), choisir la meilleure source gratuite et compatible usage commercial : vidéo → mouvement,
bibliothèques mocap, procédural/physique, keyframé.

## État
- En cours : inventaire de l'existant (agent Explore) ; recherche des sources gratuites et de
  leurs licences (agent de recherche) ; essai RTMW sur les vidéos NT14 (lot RT,
  `docs/wip/rt-rtmw.md`, branche en worktree).
- Contraintes déjà décidées : rien qui dépende de SMPL/AMASS/HumanML3D (non commercial) ;
  vidéos personnelles hors dépôt (`~/dev/cent-ans-mocap-src/`) ; pas d'extraits de films sous
  droits ; batailles navales sans 3D.

## Prochaine étape
Synthèse : tableau famille → méthode actuelle → meilleure source → lots proposés, puis
document de conception `docs/design/` et ADR.
