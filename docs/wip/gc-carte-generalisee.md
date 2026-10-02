# GC — carte généralisée (objets grossis, positions vraies)

ADR 0158. Worktree `../gp-gc`, branche `feat/gc` (pyramide en lien symbolique vers main, dylib
copiée de main : aucun changement Rust prévu).

## Mandat (joueur, 02/10)
« Je sacrifie la vue rapprochée, carte lisible à hauteur de jeu. » Paris, la Seine, etc. agrandis,
géographie relative vraie. Origine : champs (HB3 ×3) et camps plus gros que les villes 1:1.

## Lots
- [x] GC0 : diagnostic, ADR 0158, worktree, inventaire des points d'échelle (ci-dessous).
- [ ] GC1 : prototype du grossissement des villes (un facteur réglable) + captures à hauteur de
      jeu pour choisir facteur, loi compressive et plancher de caméra. **Jugement du joueur.**
- [ ] GC2 : villes — tous les niveaux (plan, F1, F2), sol, masques, clic, anneau, étiquettes,
      zones d'armée.
- [ ] GC3 : collisions — lieux absorbés en faubourgs.
- [ ] GC4 : fleuves et routes élargis, ponts.
- [ ] GC5 : champs (HB3 par rapport aux villes), arbres, hameaux, moulins, figurants.
- [ ] GC6 : plancher de caméra, retrait des paliers vallée/site, ménage (ZG5b, herbe 1:1), banc.
- [ ] GC7 : camps (avec SA, ADR 0156).
- [ ] GC8 : tests, `godot-map.md`, mémoire.

## Coordination
- SA (`../gp-sa`, ADR 0156) refait l'échelle des pions d'armée : ne pas toucher `army_markers.gd`.
- RF (`../gp-rf`) cuit E3 partout + E4 autour des villes : E4 devient peu utile (au joueur de dire
  dans cette session-là).

## Inventaire des points d'échelle
(à remplir)

## Prochaine étape
GC1.
