# PO — Polish (orchestration) — fichier de reprise

Demande du joueur (27/09) : « comment rendre le jeu plus joli et moins "amateur" ? »
Choix : plan global par rendement (D), refonte de la disposition de l'UI autorisée (A), tranche verticale.

- Spec : `docs/superpowers/specs/2026-09-27-po-polish-design.md` (validée)
- Plan : `docs/superpowers/plans/2026-09-27-po-polish.md` (validé, **source de vérité des étapes**)
- ADR à écrire : **0097** (0096 = AN1). Budget : 0 $, enveloppe ≤ 3 $.

## Reprendre dans une nouvelle session

Coller ce prompt :

> Reprends le chantier PO (polish). Lis `docs/wip/po.md`, puis la spec et le plan qu'il cite.
> Fais les vérifications d'avant reprise, puis continue à la première case non cochée de la
> liste. Tu es l'orchestrateur : tu fais PO0 et PO6 toi-même, et tu lances les lots de la vague 1
> en agents `cent-ans-dev` en worktree avec les briefs de ce fichier. Mets à jour ce fichier à
> chaque étape.

## Vérifications d'avant reprise (à chaque reprise)

1. `git status` et `git log --oneline -10` : la session a démarré sur `2c68ab91` / plan `docs: PO polish implementation plan…`.
2. `git worktree list` et `git branch --list 'feat/po*'` : retrouver les lots déjà lancés.
3. Lire l'état des chantiers voisins, qui peuvent bloquer ou décaler des étapes :

| Chantier | Fichier | Ce qui compte pour PO |
|---|---|---|
| CB (contrôles de bataille) | `docs/wip/cb.md` | CB-M1 fusionné ? Si non, PO4 étape 6 (style du marqueur de sélection) et la partie bataille de la caméra de PO5 attendent. |
| AN1 / PR1 | `docs/wip/an1-animation-vivante.md` | Possèdent le shader des soldats (sommets), les clips, les étendards et les accessoires. PO4 n'y touche pas. |
| HL (liste des colonies) | `docs/wip/colonies-liste.md` | Le panneau HL2 rejoint `UiLayout.SIDE_PANEL` (note à poser en PO0 étape 4). |
| CV3 | `docs/wip/cv3-campagne-vivante.md` | La fenêtre de rencontre, l'avis de résultat et le badge de posture sont migrés par PO1. Vérifier qu'aucun lot CV3 ne les modifie en même temps. |

4. Nombre d'agents actifs sur le dépôt : 6 au plus au total (règle CLAUDE.md). Si d'autres sessions en font déjà tourner, lancer la vague 1 en deux fois : d'abord PO2, PO3 et PO4, puis PO1 et PO5.

## Règles de travail (rappel, voir aussi la mémoire du projet)

- Commits avec des chemins explicites (`git commit -- <chemins>`) : l'index est partagé entre les sessions. Jamais de `git stash`.
- Fusions dans un worktree séparé, puis ff-only vers `main`.
- Cible cargo privée par worktree (non nécessaire ici : PO ne touche pas `core/`). Faire `godot --headless --path game --import` après chaque copie de dylib ou fusion qui ajoute des assets.
- Captures : scripts `*_shot.gd` seulement, 3 lectures d'image au plus par lot, jamais par les sous-agents.
- Supprimer ses worktrees après fusion (disque).

## Liste à cocher

### PO0 — Fondations (orchestrateur, dans `main`)
- [x] `game/tests/po_shot.gd` + planche `docs/img/po/avant/` (10 vues) — commit `wip: PO0 before board`
- [x] Inventaire des défauts par vue (section ci-dessous), chaque défaut rattaché à un lot
- [x] Bible DA § 12 « Gabarit d'interface et d'étalonnage »
- [x] `docs/decisions/0097-gabarit-interface-et-etalonnage.md`
- [x] Squelette : autoload `UiLayout`, `Settings.is_dev()` + `--dev`, `po_ui_test.gd` et `po_grade_test.gd` désactivés, `time_of_day` dans le schéma d'`atmosphere.json`, section « Polish » de `docs/budget.md`
- [x] `smoke.gd` OK — commit `PO0: foundations (bible §12, ADR 0097, UiLayout skeleton)`
- [x] `git branch feat/po-polish main` ; note `UiLayout` dans les wip HL et CV3

### Vague 1 (agents en worktree, branches issues de `feat/po-polish`)
- [x] PO2 lancé (27/09 ~21 h, Sonnet) — branche `feat/po2-typo` — worktree : agent `.claude/worktrees/agent-…` (voir `git worktree list`)
- [x] PO1 lancé (27/09 ~22 h) — branche `feat/po1-layout` — worktree : agent (voir `git worktree list`)
- [x] PO3 lancé (27/09 ~21 h) — branche `feat/po3-map` — worktree : agent (voir `git worktree list`)
- [x] PO4 lancé (27/09 ~22 h, étape 6 autorisée : CB-M1 fusionné) — branche `feat/po4-battle`
- [x] PO5 lancé (27/09 ~22 h) — branche `feat/po5-motion`

### PO6 — Intégration (orchestrateur)
- [x] Fusion de PO2 dans `feat/po-polish` (tests OK : smoke, C3 = [14, 17, 20, 26])
- [x] Fusion de PO1 (738f9fee ; bible § 12.1 mise aux mesures)
- [x] Fusion de PO3 (a8777b7f, avant PO1 : aucun fichier d'UI commun)
- [ ] Fusion de PO4 (étape 6 faite ou reportée : …)
- [x] Fusion de PO5 (925d5ae0 ; `SceneFader` en façade statique, pas d'autoload)
- [ ] Tous les tests PO actifs, pytest, `smoke.gd`, bancs PB1 carte et bataille dans ±5 %
- [ ] Planche `docs/img/po/apres/` + `docs/img/po/planche_avant_apres.jpg`
- [ ] **Jugement du joueur** (une seule fois) → corrections en PO6b si besoin
- [ ] ff de `feat/po-polish` vers `main`, worktrees et branches supprimés, `docs/manuel.md` (section « Interface »), mémoire mise à jour

### Phase 2 (après validation) — lots Sonnet
- [ ] P2a cour, fiche personnage, arbre familial
- [ ] P2b techniques, diplomatie
- [ ] P2c codex, encyclopédie, infobulles riches
- [ ] P2d sièges et résultat naval
- [ ] P2e menus secondaires
- [ ] P2f `add_theme_font_size_override` restants (179 au départ)

## Briefs des agents (vague 1)

Tronc commun à placer en tête de chaque brief :

> Tu travailles sur le jeu Cent Ans (lire `CLAUDE.md`). Chantier PO (polish) : spec
> `docs/superpowers/specs/2026-09-27-po-polish-design.md`, plan
> `docs/superpowers/plans/2026-09-27-po-polish.md`. Ton lot est la section « PO<N> » du plan :
> suis ses étapes dans l'ordre, sans dépasser la liste de fichiers du lot. Branche
> `feat/po<N>-<objet>` créée depuis `feat/po-polish`, dans ton worktree. Commence par le
> squelette et commite-le. Commits `wip:` au plus toutes les 15 min, avec des chemins explicites,
> et `docs/wip/po<N>-<objet>.md` (état, prochaine étape) mis à jour à chaque commit. Pas de
> `git stash`. Ne touche jamais `main`, ne fusionne rien : l'orchestrateur fusionne. N'utilise
> aucun outil de capture ni d'éditeur : les captures passent par ton script `*_shot.gd`, et tu
> n'ouvres pas les images. Avant de rendre la main, fusionne `feat/po-polish` dans ta branche,
> lance `smoke.gd` et les tests de ton lot, puis réponds en 10 lignes au plus : fait, reste,
> commit final, tests.

Spécifique à chaque lot :

| Lot | Modèle | À ajouter au brief |
|---|---|---|
| PO2 | Sonnet | « Ne remplace les tailles de police que dans les scripts de la tranche listés. Compte les occurrences restantes et note-les dans ton wip. Sons CC0 seulement, avec `SOURCE.md` et `CREDITS.md`. » |
| PO1 | session | « `UiLayout` ne doit jamais dépendre de la taille minimale des enfants (boucles de mise en page vues en UI1). Ne touche ni à la sélection ni aux ordres en bataille (propriété de CB). Si `UiMotion` (PO2) est absent, replie-toi sur `hide()`. » |
| PO3 | session | « Saturation ≤ 35 % à vérifier avec l'outil DA7b. Dé-encombrement DA7d inchangé. Banc PB1 carte ≤ +5 %. Pas de nouvel asset. » |
| PO4 | session | « Interdits : shader des soldats et clips (AN1), accessoires (PR1), sélection et ordres (CB). Le décalage des rangs est un décalage de rendu, sans aucun effet sur l'état du core. L'étape 6 n'est faite que si `docs/wip/cb.md` indique CB-M1 fusionné ; sinon, la noter comme reportée. Banc PB1 bataille ≤ +5 %. » |
| PO5 | session | « Lis `docs/wip/cb.md` : si CB-M1 n'est pas fusionné et modifie `battle_camera.gd`, ne fais que la caméra de campagne. Paramètres de caméra dans `data/ui/camera_feel.json` avec un schéma, jamais en dur. » |

## Inventaire des défauts (PO0, planche `docs/img/po/avant/`, 1280×720)

Diagnostic de départ (spec § 1), puis vue par vue :

| Vue | Défaut | Lot |
|---|---|---|
| carte + UI | fenêtres flottantes qui se chevauchent, texte petit, pas de hiérarchie | PO1, PO2 |
| carte + UI | message technique « Relief rapproché limité… `uv run`… » affiché au joueur | PO1 |
| carte | vert plat, brume grise, forêts en boules identiques, pastilles blanches des étiquettes | PO3 |
| bataille | lumière de midi dure, herbe en taches, sol en basse résolution, rangs raides | PO4 |
| bataille | cadre de sélection jaune vif (CB-M1 l'a remplacé par des contours ; style à reprendre) | PO4 étape 6 |
| bataille | arbres « sucette » à l'horizon | PO4 étape 5 |
| 01 titre | colonne de boutons coupée en bas à 720 px (« Crédits », « Quitter » hors écran) | PO1 |
| 01 titre | libellés du menu sans état de survol marqué, légende du plan minuscule | PO2 |
| 02 faction | textes des cartes et des forces/faiblesses sous 12 px ; boutons de difficulté plats | PO2 |
| 02 faction | titre « Choisissez votre couronne » collé au bord, sans marge de grille | PO2 |
| 03 carte large | barre du haut : texte minuscule, icônes et chiffres serrés, sans hiérarchie | PO1, PO2 |
| 03 carte large | minicarte ≈ 1/4 de la hauteur, boutons de filtres collés dessous | PO1 |
| 03 carte large | conseil (haut gauche), journal (bas gauche) et cloche (bas droite) flottent chacun dans leur coin | PO1 (toasts) |
| 03 carte large | étiquettes de villes en pastilles, vert plat, lumière neutre | PO3 |
| 04 régionale | panneau de province haut de 90 % de l'écran, collé à la minicarte, texte 11-12 px | PO1, PO2 |
| 05 armée | libellé du chemin (« ce point : 4 tours… ») flotte au milieu du bas et chevauche le journal | PO1 |
| 05 armée | carte du général et bande d'armée se chevauchent avec le journal | PO1 |
| 06 fin de tour | bandeau « Tour des autres factions » posé sur le panneau de province (deux panneaux à droite) | PO1, PO5 |
| 07 rencontre | fenêtre de choix sans fond assombri, recouvre la minicarte ; bande d'armée visible dessous | PO1 (modal) |
| 08 déploiement | six panneaux flottants (en-tête, journal, bandeau, alerte rouge, ordres, cartes) | PO1 |
| 08 déploiement | sol plat et uniforme, lumière de midi, pas d'ombres portées lisibles | PO4 |
| 09 mêlée | journal de bataille en grand panneau haut droite, texte serré | PO1, PO2 |
| 09 mêlée | champ vert uniforme, rangs en blocs rigides, horizon sans profondeur | PO4 |
| 10 résultat | page dense, petites tailles, tableaux sans respiration | PO2 (phase 2 pour le détail) |
| toutes | aucune transition : les écrans apparaissent d'un coup | PO5 |

## Journal

- 27/09 : brainstorming terminé ; spec validée par le joueur.
- 27/09 : spec corrigée (LUT procédurale existante réutilisée, ADR 0097 car 0096 = AN1, AN1/PR1 exclus de PO4) ; plan écrit et validé.
- 27/09 : fichier de reprise rédigé. **Prochaine étape : PO0, première case.** Rien n'est lancé, aucun worktree PO.
- 27/09 : PO0 commencé ; planche « avant » (10 vues JPEG, `po_shot.gd` pilote les options de capture des scènes) et inventaire faits. Constat : CB-M1 et CV3-4 sont fusionnés ; 4 agents d'autres chantiers tournent (AN1a, AN1b, PR1, CB-M2), donc 2 lots PO à la fois au plus.
- 27/09 : PO0 fondations : bible § 12 (zones en parts d'écran, échelle Title 26 / Heading 20 / Body 17 / Caption 14 px à 900 px de référence, soit ×1,2 à 1080p — ancrée sur la taille par défaut du thème, 17), ADR 0097, squelette (`UiLayout`, `Settings.is_dev()` + `--dev`, tests PO désactivés, `time_of_day` au schéma, budget). Smoke vert.
- 27/09 : PO0 fini (c6cc7cf1), branche `feat/po-polish` créée (eb6a8a23). Vague 1 en deux temps (4 agents d'autres chantiers actifs : AN1a, AN1b, PR1, CB-M2) : PO2 et PO3 lancés ; PO4, PO1 (après la fusion de PO2), puis PO5 à mesure que des places se libèrent. **Prochaine étape : attendre PO2/PO3, fusionner PO2, lancer PO4 et PO1.**
- 27/09 : PO2 rendu et fusionné dans `feat/po-polish` (a7133864) ; `main` (AN1a/AN1b) fusionné dans `feat/po-polish` (143b3ced), smoke vert. Restes PO2 confiés à PO1 (tailles figées des `.tscn` carte et province). 162 surcharges de taille hors tranche → phase 2. Worktree de fusion de l'orchestrateur : `scratchpad/po-merge` (branche `feat/po-polish`). AN1 clos : PO1, PO4, PO5 lancés. **Prochaine étape : fusionner PO1 → PO3 → PO4 → PO5 à leur retour.**
- 27/09 : PO3 fusionné dans `feat/po-polish` (a8777b7f). Verts : smoke, po_grade (campagne), C3, da7d, pytest 787. `sz4b_colonies_forests_test` échouait déjà avant PO3. À faire en PO6 : **banc PB1 carte à remesurer sur machine calme** (mesures PO3 bruitées, vue 150 à +5 %) ; saturation ramenée à ≤ 35 % (carte un peu plus terne : à soumettre au joueur en C5).
- 27/09 : PO5 fusionné dans `feat/po-polish` (925d5ae0). Verts : smoke, po5_motion, zg4_camera, trackpad_zoom, po_grade, po_ui, pytest 789. Restes : pas de voile à l'entrée en bataille (l'écran de chargement fait la transition) ; minicarte de bataille en coupe franche. PO1 prévenu : bandeau U5 sans la date (cartouche de saison PO5). En cours : PO1, PO4.
- 27/09 : PO1 fusionné dans `feat/po-polish` (738f9fee). Verts : po_ui (UiLayout, C1 128 textes, C2 52 contrôles, C3), smoke et 15 tests UI/caméra/CB. Écarts acceptés et reportés dans la bible § 12.1 : TOP_BAR 0,08 ; cloche hors zone ; fins de bataille/partie en écrans pleins. Restes : bande d'ost dépasse de 10 px à 720p (phase 2 ou PO6b). En cours : PO4.
