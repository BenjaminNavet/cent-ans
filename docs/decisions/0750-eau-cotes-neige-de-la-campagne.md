# 0750 — Eau, côtes et neige de la campagne

Statut : accepté (lot EAU, 23/10/2026). Numéro choisi hors de la plage 0748-0749 pour éviter une collision avec les lots LUM et TERRAIN menés en parallèle.

## Contexte

Le juge DA (itération 02, `out/juge/iter-02-art.md`, constats 8 à 10) relevait : mer sombre « huile » avec un
motif de crêtes blanches répétitif, côte en escalier avec frange cyan pixélisée, rivières à bord net, neige en
pâtés crénelés, forêts d'hiver en blocs bruns, ombres de nuages bleues trop grandes.

Causes trouvées dans le code :

- les « vermicelles » ne viennent pas des textures de détail (houle grise sans motif) mais de `sea_crests` :
  lignes `smoothstep(sin(phase))` posées sur un fond presque noir ;
- la côte est lue dans `coast_dist.png` (L8, un texel par pixel carte, pas de 0,5 px) en bilinéaire : de près
  (plusieurs pixels écran par pixel carte) la ligne de rivage, l'écume et la plage dessinent des marches ;
  la carte composée est une donnée liée hors dépôt (`docs/editeur-carte.md`), elle n'est pas régénérée ;
- la profondeur est estimée à 12 m par pixel de côte : le dégradé turquoise → large tient en 6 pixels ;
- la neige est un seuil `smoothstep(0,42, 0,68)` sur du bruit, sans exposition.

## Décision

Tout est rendu seulement, piloté par les données, avec un interrupteur de comparaison.

1. **Mer** (`sea.wesl`, `data/fx/sea_life.json` bloc `eau`, `sea_life.rs::apply_eau`) : couleurs de profondeur
   plus claires et plus bleu-vert, profondeur estimée à 4,5 m par pixel (dégradé turquoise sur ~15 px),
   fonds irréguliers (bruit sur la profondeur), saturation maximale plus haute près du rivage, houle à deux
   trains supplémentaires déformés par un champ lent, moutons doux (champ étiré dans le vent, deux échelles de
   déformation) à la place des lignes de crêtes, largeur d'écume irrégulière le long du rivage.
2. **Côte** : lecture B-spline cubique (quatre lectures bilinéaires, même échantillonneur, aucune liaison
   ajoutée) du champ de distance, seulement à moins de 5 px du rivage et sous 1,8 px carte par pixel écran,
   dans `sea.wesl` et dans `terrain.wesl::coast_px_smooth` (plage, falaise, écume au sol) : les deux couches
   voient la même ligne. La limite de 16 échantillonneurs de Metal (`docs/wip/bevy-camp-samplers.md`) est
   respectée.
3. **Rivières** (`river.wesl`, `river_display.json`, `water/season.rs`) : bord adouci (`edge_soft_px`,
   encre de berge), reflet de base plus fort, teinte de l'eau par saison et glace de berge en hiver,
   synchronisées sur `SeasonWeights`.
4. **Neige** (`terrain.wesl`) : accumulation par altitude et exposition au soleil (la neige fond d'abord sur
   les faces tournées vers lui), rampe large au lieu d'un masque net, bruit moins ample, couvert forestier
   givré (plus gris-bleu, la canopée reste visible) ; ombres de nuages plus petites (échelle 0,058), plus
   douces (bord 0,22), moins sombres (0,30) et moins bleues (`data/ui/campaign_map.json`).

`CENT_ANS_EAU_V2=0` coupe les nouveautés de la mer (couleurs, moutons, houle, lissage de côte en mer) pour
`tools/bevy_bench/ab.sh`.

## Conséquences

- Coût : quatre lectures de texture de plus par pixel de mer ou de sol à moins de 5 px du rivage et de près ;
  rien ailleurs. Mesure dans `docs/wip/eau-cotes-neige.md`.
- Écarts : les rivières fines (`river_fine.wesl`) ne reçoivent que le bord et le reflet partagés par les
  données, pas la teinte saisonnière ; la forêt d'hiver garde la couleur de sol du lot TERRAIN.
- Limite : le lissage arrondit un peu les caps de moins de 2 px (flou de 1 texel du B-spline).
