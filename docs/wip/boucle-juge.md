# Boucle juge → corrections

Mandat 23/10 : boucle illimitée (arrêt par le joueur, ou deux passes de suite sans constat majeur).
Juge = sous-agent Sonnet indépendant, rapport dans `out/juge/iter-NN.md`. Corrections par lots, commits ciblés, pas de push.
Axes : jouabilité, gameplay, visuels, assets, couleur, textures, fps (cible 60 en Ultra), cohérence, véracité historique et géographique,
UI/UX, prise en main, équilibrage, accessibilité, stabilité, chargement/mémoire, aberrations visuelles, audio (logs/fichiers), textes/localisation,
échelle, cohérence artistique, données, authenticité matérielle, plausibilité militaire et politique.
Budget : Meshy 155 crédits (23/10, à choisir avec soin, confirmer le coût avant), 2D via Z-Image. Aucune autre dépense cloud.

## Itérations
| # | Juge | Majeurs | Corrigés | fps | Commit |
|---|------|---------|----------|-----|--------|
| 1 | out/juge/iter-01.md | 1 (C1 fps bataille) → infirmé par banc neuf `j1` | 0 code ; rejetés : C1 (bataille 75 fps calme / 59 mêlée, p95 18,5 ms), C2 (ADR 0183), C3 (météo brouillard ADR 0596), C5 (champ `playable` = sélecteur de faction, 139 voulues), C6, C8 (déjà daté 1360), C10 (artefact `--offscreen`) ; restent C4 (toits de siège sombres, mineur), C7 (conseiller Jean le Bel, mineur) | carte pan 69 moy., bataille 75/59 | 9518d5d0b |

## Prochaine étape
Itération 2 : juge ciblé sur gameplay/IA (parties scriptées, seed bots), tutoriel, mémoire, plausibilité ; constats à prouver (dump/mesure) avant d'être retenus. Faux positifs connus à éviter : voir ligne 1.

## Intégration 23/10 (branche integ/juge)
Fusion des lots MEM, HIST, DAUPHINE, ECO, CAUSES, UX, LUM, EAU, TERRAIN (partiel), ARMEES (partiel). ADR renumérotés : 0754 causes, 0755 palette.
Dérogation décidée par le joueur pour CE tour : ni smoke ni seed bots. Contrôle = clippy, tests cœur et client, pytest. Smoke et seed bots à passer au prochain tour d'intégration.
Restes : hameaux + ADR terrain, densité/imposteurs des armées, vérification visuelle campagne (LUM, EAU), 2 échecs pytest HIST (maisons sans blason, références d'archétypes de portraits), refontes de design (IA à doctrines, arcs de partie).

## Reprise (état au 23/10 soir)
FAIT : les 10 lots sont dans main (09eb6e7dd, ff-only, rien de poussé) ; worktrees des lots et branches supprimés. Vert : clippy + tests cœur, clippy + tests client (dont real_table_and_glbs sur main), pytest 2920. Dérogation du tour : ni smoke ni seed bots → À PASSER au prochain tour.
Correctifs d'intégration : seeds de tests (m4 seed 13, f7 seed 1341 : le lot HIST a déplacé la dynamique simulée), STATE_VERSION 10, ADR renumérotés (0754 causes, 0755 palette), starting_fit régénéré, schémas/données Dauphiné/Dunbar/Castille.
Outils : `tools/heavy.sh` (sémaphore 2 emplacements), `tools/full_check.sh [dossier]` (clippy + tests cœur/client).
PROCHAINE ÉTAPE : itération 3 du juge (4 angles : DA, design, histoire/UX, gameplay/IA ; captures campagne en `CENT_ANS_WEATHER=clear`, fps sur machine calme) + smoke + seed bots sur main.
Restes : hameaux + ADR terrain, densité/imposteurs/premier plan des armées (ARMEES partiel), vérification visuelle campagne (LUM, EAU : captures campagne inexploitables sous charge), refontes de design (IA à doctrines, arcs de partie, scénarios), pistes mémoire (copies CPU, KTX2 sol), données de faction Dauphiné (barks etc.).
DISQUE : 19 Go libres (98 %) ; client/target = 57 Go (main), core/target 6,9 Go ; snapshots Time Machine locaux (3). Ne rien supprimer sans accord (voir mémoire disque).
