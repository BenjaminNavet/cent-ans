# VIS-R4 : montagne étagée et neige vraie (état)

Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R4 ; ADR 0820. Branche : worktree-agent-a47dbd6c148e66466 (rebasée sur main a12294888). Non fusionné, non poussé.

## Fait
- Données : bloc `mountain` de `data/ui/campaign_seasons.json` (limite des arbres sud/nord, adret/ubac, forêt de pente, alpage, lande du nord, éboulis, neige d'altitude 0,90 0,92 0,95 / ombre 0,62 0,68 0,80) + schéma ; `snow.plain_share` 0,93 -> 0,25 (drap blanc seulement par météo) ; `ground_look.rock_color` 0,11 -> 0,42 (granite de la bible) ; glaciers `map_freshwater.json` (ligne 2450 m, glace plus claire, pente max 0,68).
- `season.rs` : 10 vec4 de plus dans l'uniforme `SeasonParams` (`mt_*`), lecture `read_mountain`, test des données.
- `terrain.wesl` : `mountain_stages` (ALU seulement, 2 vnoise, aucun échantillonneur), `alpine_share`, `snow_tint(.., alpine)`, névé en blanc.
- Brume de vallée : plafond par temps clair déjà bas (`clear_max` 0,16, VIS-R1) : inchangée.

## Mesures HSV (captures 1280x800, recadrage central hors HUD ; script hors dépôt)
Neige (V > 0,78 et S < 0,28) : part de l'image / V médian :
- Alpes d 400 : 15,3 % -> 21,4 % ; V 0,91 -> 0,93
- Alpes d 120 : 26,2 % -> 40,2 %
- Pyrénées d 300 : 7,0 % -> 11,0 %
- Highlands d 400 : 4,4 % -> 4,5 % (printemps, ligne de neige à 2 000 m : pas de neige attendue)
Teinte de la neige à l'écran reste chaude (40-49 deg, S 0,14-0,19) : lumière du soleil 5 000 K + brume chaude ; l'albédo est bien 0,90 0,92 0,95. Roche (S < 0,3) : teinte 95-106 deg, S 0,14, V 0,50-0,53 (gris neutre, plus de crème rosé).

## Vérifs
shader_check, fmt, clippy `--features dev --all-targets -D warnings`, tests client 1535 verts, pytest schéma saisons. Seul échec : `snapshots::smoke_scenarios_parse_to_stable_command_lists` (hérité de main, hors zone). 3 captures utilisées.

## Reste / points ouverts
- Coût GPU non mesuré (banc A/B `tools/bevy_bench/ab.sh` non lancé, machine chargée) ; estimé quelques dizaines d'ALU par pixel de sol.
- Fond de vallée vert : la carte de couleur le fournit déjà, pas de passe dédiée (sans masque de montagne, elle recolorerait les plaines).
- Neige hivernale de plaine non revue à l'image (seulement `plain_share`).
- Highlands : lande brun-violet de l'alpage du nord non contrôlée à l'image.
Spec : docs/audits/2026-10-29-visuel-campagne.md, lot R4 ; ADR 0820. Branche : worktree-agent-a47dbd6c148e66466.

## Fait
- Données : bloc `mountain` de `data/ui/campaign_seasons.json` (limite des arbres sud/nord, adret/ubac, forêt de pente, alpage, lande du nord, éboulis, neige d'altitude 0,90 0,92 0,95 / ombre 0,62 0,68 0,80) + schéma ; `snow.plain_share` 0,93 -> 0,25 (drap blanc seulement par météo) ; `ground_look.rock_color` 0,11 -> 0,42 (granite de la bible) ; glaciers `map_freshwater.json` (ligne 2450 m, glace plus claire, pente max 0,68).
- `season.rs` : 10 vec4 de plus dans l'uniforme `SeasonParams` (`mt_*`), lecture `read_mountain`, test des données.
- `terrain.wesl` : `mountain_stages` (ALU seulement, 2 vnoise, aucun échantillonneur), `alpine_share`, `snow_tint(.., alpine)`, névé en blanc.
- Brume de vallée : plafond par temps clair déjà bas (`clear_max` 0,16, VIS-R1) : inchangée.

## Mesures HSV (captures 1280x800, recadrage central hors HUD ; script hors dépôt)
Neige (V > 0,78 et S < 0,28) : part de l'image / V médian :
- Alpes d 400 : 15,3 % -> 21,4 % ; V 0,91 -> 0,93
- Alpes d 120 : 26,2 % -> 40,2 %
- Pyrénées d 300 : 7,0 % -> 11,0 %
- Highlands d 400 : 4,4 % -> 4,5 % (printemps, ligne de neige à 2 000 m : pas de neige attendue)
Teinte de la neige à l'écran reste chaude (40-49 deg, S 0,14-0,19) : lumière du soleil 5 000 K + brume chaude ; l'albédo est bien 0,90 0,92 0,95. Roche (S < 0,3) : teinte 95-106 deg, S 0,14, V 0,50-0,53 (gris neutre, plus de crème rosé).

## Vérifs
shader_check, fmt, clippy `--features dev --all-targets -D warnings`, tests client 1535 verts, pytest schéma saisons. Seul échec : `snapshots::smoke_scenarios_parse_to_stable_command_lists` (hérité de main, hors zone). 3 captures utilisées.

## Reste / points ouverts
- Coût GPU non mesuré (banc A/B `tools/bevy_bench/ab.sh` non lancé, machine chargée) ; estimé quelques dizaines d'ALU par pixel de sol.
- Fond de vallée vert : la carte de couleur le fournit déjà, pas de passe dédiée (sans masque de montagne, elle recolorerait les plaines).
- Neige hivernale de plaine non revue à l'image (seulement `plain_share`).
- Highlands : lande brun-violet de l'alpage du nord non contrôlée à l'image.
