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
08/10 : AS1-AS5 FUSIONNÉS dans main (8b669767e), worktrees supprimés ; smoke + as1..as5_test
verts ; pytest : 1 échec antérieur sans rapport (`test_relief_update`). ADR 0188 (AS1) ;
0189-0192 libres. Planches AS1 regardées (pattes alternées au pas, chevaux du camp qui broutent :
correct) ; `as5_shot.gd` inutilisable (scène sans lumière, flammes minuscules) ; AS2-AS4 non vus.

Restes :
- AS7 jugement en jeu (joueur) : AS2 cadence carte, AS3 trot/virage (sens du virage à
  confirmer), AS4 positions des tas et chevauchement porteur/chargeur, AS5 flammes et bannières.
- Lot de calage (AS8) : mesurer rythmes/amplitudes sur `docs/research/as-references-video.md`
  et régler `data/fx/animal_motion.json`, `campaign_army_walk.json`, `battle_animation.json`
  (`cavalry_gaits`), `siege_engines.json` (`crew.haul`), `map_fire_wind.json`.
- Bivouacs en feu sur la carte (liste des armées à l'arrêt), porte-étendard monté au trot
  (`battle_standards.gd`, clip `c_std_trot` déjà cuit), ombre des imposteurs d'arbres fixe,
  `as5_shot.gd` à éclairer et cadrer.
- AS6 tournage (joueur).
