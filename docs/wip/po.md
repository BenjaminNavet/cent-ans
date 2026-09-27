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
- [ ] `game/tests/po_shot.gd` + planche `docs/img/po/avant/` (10 vues) — commit `wip: PO0 before board`
- [ ] Inventaire des défauts par vue (section ci-dessous), chaque défaut rattaché à un lot
- [ ] Bible DA § 12 « Gabarit d'interface et d'étalonnage »
- [ ] `docs/decisions/0097-gabarit-interface-et-etalonnage.md`
- [ ] Squelette : autoload `UiLayout`, `Settings.is_dev()` + `--dev`, `po_ui_test.gd` et `po_grade_test.gd` désactivés, `time_of_day` dans le schéma d'`atmosphere.json`, section « Polish » de `docs/budget.md`
- [ ] `smoke.gd` OK — commit `PO0: foundations (bible §12, ADR 0097, UiLayout skeleton)`
- [ ] `git branch feat/po-polish main` ; note `UiLayout` dans les wip HL et CV3

### Vague 1 (agents en worktree, branches issues de `feat/po-polish`)
- [ ] PO2 lancé — branche `feat/po2-typo` — worktree : …
- [ ] PO1 lancé — branche `feat/po1-layout` — worktree : …
- [ ] PO3 lancé — branche `feat/po3-map` — worktree : …
- [ ] PO4 lancé — branche `feat/po4-battle` — worktree : …
- [ ] PO5 lancé — branche `feat/po5-motion` — worktree : …

### PO6 — Intégration (orchestrateur)
- [ ] Fusion de PO2 dans `feat/po-polish` (tests OK)
- [ ] Fusion de PO1
- [ ] Fusion de PO3
- [ ] Fusion de PO4 (étape 6 faite ou reportée : …)
- [ ] Fusion de PO5
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

## Inventaire des défauts (à remplir en PO0)

Diagnostic de départ (spec § 1), à compléter vue par vue :

| Vue | Défaut | Lot |
|---|---|---|
| carte + UI | fenêtres flottantes qui se chevauchent, texte petit, pas de hiérarchie | PO1, PO2 |
| carte + UI | message technique « Relief rapproché limité… `uv run`… » affiché au joueur | PO1 |
| carte | vert plat, brume grise, forêts en boules identiques, pastilles blanches des étiquettes | PO3 |
| bataille | lumière de midi dure, herbe en taches, sol en basse résolution, rangs raides | PO4 |
| bataille | cadre de sélection jaune vif | PO4 étape 6 (après CB-M1) |
| bataille | arbres « sucette » à l'horizon | PO4 étape 5 |

## Journal

- 27/09 : brainstorming terminé ; spec validée par le joueur.
- 27/09 : spec corrigée (LUT procédurale existante réutilisée, ADR 0097 car 0096 = AN1, AN1/PR1 exclus de PO4) ; plan écrit et validé.
- 27/09 : fichier de reprise rédigé. **Prochaine étape : PO0, première case.** Rien n'est lancé, aucun worktree PO.
