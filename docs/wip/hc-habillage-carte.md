# HC — habillage de la carte de campagne (forêts, lacs, champs)

Worktree `../gp-hc`, branche `feat/hc` (pyramide en lien symbolique vers main, dylib copiée de
main). ADR réservé : 0161.

## Mandat (joueur, 02/10)
« Je veux plus de forêts, lacs, champs etc. pour habiller la carte de campagne. »

## Constat (captures `~/.cache/cent_ans/tb/ours/avant-sol-90.png` et `-400.png`, Paris)
- Forêts : quelques grandes taches vert sombre **plates**, aucun volume d'arbres à hauteur de jeu.
- Lacs : aucun visible (762 dans `data/map/lakes.json`, seuil 30 px ; étangs absents).
- Champs : parcellaire partout, uniforme ; pas de vignes, vergers, marais ni landes lisibles.

## Coordination
- GC (`../gp-gc`, ADR 0158) : carte généralisée, objets grossis. **GC5 règle les champs**
  (`../gp-gc-fields`, `field_scale` dans `terrain.gdshader`) : HC ne touche pas au parcellaire du
  shader de terrain tant que GC5 n'est pas fusionné.
- SA (`../gp-sa`) : ne pas toucher `army_markers.gd`.
- TB (`../gp-tb*`) : saisons, traces de guerre ; pas de recouvrement prévu.

## Lots
- [x] HC0 : constat, worktree, inventaires (rendu Godot, pipeline geo).
- [ ] Plan des lots (à écrire au retour des inventaires).

## Prochaine étape
Lire les deux inventaires, écrire le plan des lots et l'ADR 0161, lancer la première vague.
