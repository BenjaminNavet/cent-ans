# Lot EAU : mer, côtes, rivières, neige (campagne)

Source : `out/juge/iter-02-art.md` constats 8, 9, 10. ADR 0750.

## État
- Squelette : note.

## Plan
1. Mer (`water/shaders/sea.wesl`, `data/fx/sea_life.json` bloc `eau`) : couleurs par profondeur, houle plus riche, moutons doux à la place des lignes de crêtes (les « vermicelles » viennent de `sea_crests`, pas des textures de détail), écume de côte irrégulière.
2. Côte : lissage B-spline (4 lectures bilinéaires) du champ `coast_dist` près du rivage, en mer (`sea.wesl`) et au sol (`terrain.wesl::coast_px`), sans échantillonneur supplémentaire.
3. Rivières : bord plus doux, teinte saisonnière, léger reflet (`river.wesl`).
4. Neige : accumulation par normale / altitude, givre sur les forêts ; ombres de nuages plus petites et douces (`terrain.wesl`, `data/ui/campaign_map.json`).

## Contraintes
- Pas de nouvel échantillonneur (limite 16 sur Metal, `bevy-camp-samplers.md`).
- fps campagne : `CENT_ANS_EAU_V2=0` coupe les nouveautés de la mer pour `ab.sh`.
- Ne pas toucher `weather/atmosphere.rs`, `data/fx/atmosphere.json`, LUT (lot LUM) ni les matériaux du sol hors neige (lot TERRAIN).

## Prochaine étape
Implémenter la mer, puis la côte.
