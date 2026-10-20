# TS-T3 — atmosphère de la carte de campagne (ADR 0710)

Branche `ts/t3`, worktree `../gp-ts-t3`. Plan : `docs/wip/ts-style-total-war.md`.

## Conception
- La passe plein écran `terrain/post.rs` (après tonemapping, caméra de campagne seulement, l'UI a sa propre caméra) devient une passe maison qui lit aussi la profondeur de la vue : position monde par pixel → brume de vallée selon l'altitude, perspective atmosphérique gris-bleu, froid, puis étalonnage désaturé. S'applique au monde entier (arbres compris) sans toucher `terrain.wesl` (T1) ni `vegetation/` (T2).
- Données : `data/fx/campaign_post.json` (schéma `fx_campaign_post.schema.json`), interrupteur global `enabled` + un par bloc.
- Flocons (`weather/precip.rs`) : quads face caméra, disque flou, tailles/alpha variables, dérive.

## État
- [x] squelette (données, schéma, note)
- [x] passe profondeur + brume + aérien + froid + étalonnage (compile, tests unitaires ; rendu à vérifier)
- [x] flocons doux (quads face caméra, disque flou, `precipitation.tres` snow_*)
- [x] vérifs (fmt, clippy, 1334 tests, smoke 10/10, schéma) + captures alps_mid / alps_winter / town_winter

## Prochaine étape
Lot terminé, en attente de fusion par l'orchestrateur.

## Points ouverts
- Altitude par pixel = y monde / échelle verticale : avec l'écrasement/le soulèvement des massifs (ADR 0353) ce sont des « mètres affichés » ; la bande de brume suit l'altitude du foyer (`follow_focus`) pour rester relative.
- L'horizon « papier » de la vue moyenne (haut de alps_mid) ne vient pas de la brume : c'est le rendu de la carte au-delà du relief 3D / les étiquettes parchemin (ADR 0124), non modifié.
- Pas de banc de perfs dédié : la passe existait déjà, elle ajoute une lecture de profondeur et 2 bruits de valeur par pixel.
- Flocons peu visibles en vue 60 (taille Godot `snow_size_*`), réglables dans `precipitation.tres`.
- Pas de brume par province (froid = foyer de caméra + altitude par pixel).
