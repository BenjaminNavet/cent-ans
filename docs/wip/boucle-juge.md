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

## Itérations 3 et 4 (24/10)
Itération 3 : juges DA 4,8/10 (sol noir, corrigé fb6b6e23d), design 6,0, histoire 7,7 / UX 6,4, technique (fps OK : campagne ~95 fps, bataille ~60 ; mémoire campagne 10-12 Go → corrigée par MEM2 : 2,75 Go).
Fusionnés dans main (3000200eb, ff-only, rien de poussé) : DATA2, UX2 (garde-fous shader, ADR 0774/0775), EVENTS (ADR 0779), MEM2 (ADR 0776), ECO2 (ADR 0777/0778), FORET (ADR 0770). Contrôle complet vert : clippy+tests cœur et client, pytest 2941. Dérogation : smoke et seed bots non passés sur cette intégration (le juge technique a passé les seed bots quick avant : 0 constat).
Ajustements d'intégration : ev_history (la guerre Angleterre-France de la partie simulée finit en 1351 avec ECO2 → Maupertuis couvert par son test à état fabriqué), forest look défaut near_scale 0.8, clippy as_chunks, cathédrales retirées d'Orléans/Nidaros/Polotsk.
EN COURS : lot BATAILLE (worktree agent-a8d4b4abbbb986309 : siège, mêlée, masse, ADR 0772/0773) ; à fusionner puis banc bataille sur machine calme.
RESTES : boutons client emprunter/rembourser/lever un impôt (ECO2), haies/cours de hameaux, KTX2 sol de campagne, mémoire bataille (plancher 3,8 Go, pics IOAccelerator), IA à doctrines stratégiques, arc de partie en actes, recherche/espionnage/peste, cohérence palette campagne/bataille à rejuger (sol visible), Murcie (renommer ou créer colonie), mariage de maître de Calatrava (règle IA Rust), tributs de traité sans producteur.
PROCHAINE ÉTAPE : fusion de BATAILLE, itération 4 du juge (4 angles), smoke + seed bots.

## Itération 3 clôturée (24/10) — main c10f396e7 (rien de poussé)
Fusionnés depuis l'itération 3 : DATA2, UX2, EVENTS, MEM2, ECO2, FORET, IA (doctrines, ADR 0782/0783), DATA3, ACTES (arc en 3 actes, ADR 0784/0785, sauvegarde v11), CREDIT-UI (onglet Finances, ADR 0780), BATAILLE (ADR 0772/0773). ADR renumérotés/complétés dans INDEX.
Contrôle : clippy + tests cœur (2058) et client (1496, + real_table_and_glbs qui passe sur main avec assets), pytest 2941 (avant IA/DATA3/ACTES/CREDIT). Smoke NON passé sur ces intégrations. Seed bots quick/long passés par le lot IA (0 constat).
Perf de bataille mesurée en bissection : fb6b6e23d = 79,6/60,0 fps ; main (cbeded0c3) = 79,3/60,1 ; BATAILLE seul = 78,6/59,0 ; fusion = 78,9/60,0. Une première mesure à 46/33 dans un worktree d'intégration était un artefact d'environnement (cause non trouvée) : toujours recroiser une mesure surprenante dans un worktree frais.
Piège levé : mimalloc incompatible avec la liaison dynamique du build `dev` (corrigé : allocateur système en dev, 5a30c838a).
RESTES : sol de campagne à rejuger visuellement (après correctif shader fb6b6e23d), haies/cours de hameaux, KTX2 sol de campagne, mémoire bataille (plancher 3,8 Go, pics IOAccelerator), imposteurs animés/unit_size, tributs de traité (producteur = lot de migration, ECO2/DATA3 : doublerait le paiement), rééquilibrage du seuil de score de règne (72, France passive plafonne vers 66) avec seed bots, recherche/espionnage/peste (réseau unique), cohérence palette campagne/bataille, smoke complet.
PROCHAINE ÉTAPE : itération 4 du juge (4 angles : DA, design, histoire/UX, technique) sur main c10f396e7 ; smoke + seed bots.

## Itération 4 clôturée (24/10) — main 851cdb35e (rien de poussé)
Juges (brief minimal, libres) : DA 6,5 (passe courte), design 6,8, histoire/UX (erreurs ciblées), technique (carte 117 fps, bataille 59-60 sans marge, mémoire campagne 5,8-10,4 Go hors budget). Smoke 11/11 OK sur c10f396e7.
Fusionnés : HIST3 (règle « un vassal ne cherche plus d'alliance », textes, « le Pale »), TERRAIN2 (patchwork du sol : contraste des parcelles, ADR 0787), GAMEPLAY2 (score de règne, opulence nette de dettes, conseiller, oracle seed bots ; ADR 0788/0789). Contrôle : clippy + tests cœur et client, pytest 2946. Seuil f7_events abaissé de 20 à 18 (HIST3 retire des guerres chaînées ; scan 10 graines = 18-19).
EN COURS : MEM3 (worktree agent-a0377ad7d1eea36f9 : protocole de mesure mémoire reproductible, décomposition des ≈ 4 Go non expliqués, budget ; il attend une machine calme).
NON FAIT (itération 5 à ne pas lancer sans accord) : BATAILLE2 (fenêtres de déploiement et pastilles encombrantes, troupes lointaines en « pions », marge fps de mêlée, « Non » au didacticiel de bataille qui ne lance pas la bataille), UX3 (guide du didacticiel recouvre des panneaux, Diplomatie qui s'ouvre seule, `click_if` muet en script, écran de transition d'acte absent avec CENT_ANS_FORCE_ACT=1), DA à rejuger en profondeur (saisons, siège, nuit, Alpes), fac_hama texte, `cent-ans codex-bundle` chemin parents[2], colonies manquantes (ADR 0291 plafonne à 5 par province : options dans docs/wip/hist3-corrections.md), tributs de traité (migration), mémoire bataille.
Worktrees HIST3 et GAMEPLAY2 verrouillés par la session en cours : à supprimer après (`git worktree remove`, branches fusionnées).
