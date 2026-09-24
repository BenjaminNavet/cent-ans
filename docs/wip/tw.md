# WIP orchestrateur — rapprochement Total War (session 6)

Mandat (24/09) : autonomie complète, plusieurs heures. Rapprocher le jeu de Total War, visuellement et en mécaniques ; garder les mécaniques propres à Cent Ans.
Hors périmètre : refonte colonies (C2c-C7, autre session, `docs/wip/colonies.md`), portraits.

Choix du joueur au lancement :
- Référence : **mélange moderne** (mécaniques de Medieval II + ergonomie et visuel des TW récents : Three Kingdoms, Pharaoh, Warhammer III).
- Priorité : **équilibrée**, alterner vagues bataille / campagne.
- Autorisé : dépenses cloud dans le plafond de 50 $, assets libres CC0/CC crédités, Blender local (MCP), push auto de main après chaque vague.

## Vagues

| Vague | Lots | État |
|---|---|---|
| 0 | P1 petits points cœur (no_quarter, étain, IA Normandie ouest) ; P2 rapport de saison + smoke ; R exploration TW | **fusionné** (P2 f48b557, P1 ae4ec92) |
| 1 | B1 maillages soldats/chevaux (Blender) ; B2 bannières d'unité + vignettes + écran de fin ; C1 minicarte + brouillard | **fusionné** : B2 c0b9635, B1 ff592ae, C1 ee4f2b6 |
| 2 | B3 musique dynamique + caméra de suivi ; B4 effets + animations (suites B1) + rythme d'engagement ; C2 zone de contrôle **après C4 colonies** (refonte du déplacement) | B3 fusionné (953a758) ; B4 **fusionné** (2a485a5, banc fait 51f95be) ; C2 en attente |
| 3 | C3 arbre familial + fiche de général (arbre de compétences) ; B5 champs de bataille tirés de la campagne (biome, village, saison) | B5 **fusionné** (be47ae9) ; C3 **fusionné** (2a485a5) |
| 4 | B6 IA tactique qui exploite le site (haies, village) + libellé du terrain avant-bataille ; B7 finitions visuelles (fanions de lance, repères regroupés au loin, neige, éclaboussures de gué) ; C7 suite du général à la Medieval II + année de décès + mise en page fiche < 1500 px | B7 **fusionné** (7441f5c) ; B6 **fusionné** (e618e41) ; C7 en cours (`docs/wip/c7-suite-general.md` dans sa branche) |
| 5 | C6 agents Medieval II (espion, émissaire, prédicateur) ; C2 zone de contrôle après C7a colonies | C6 **en cours** (agent en worktree, `docs/wip/c6-agents.md`) ; C2 en attente |

## État courant (24/09, reprise après arrêt)

main poussé (e618e41). Fusionnés : P1, P2, B1-B7, C1, C3. Banc B4 fait : effets ≈ +6 % primitives, FPS inchangés (plafond 60 Hz) ; option `--bench-at=<s>`.
En cours : C7 (suite du général), C6 (agents) ; branches `worktree-agent-*` (`git worktree list`).
Colonies C4 et C5 fusionnées (17aa679, b241608) : l'aire atteignable existe déjà (`reachable_markers.gd`), C2 se réduit à la zone de contrôle, qui attend C7a colonies (équilibrage de `movement.rs`). C4 TW (édits/chaînes) et C5 TW (routes commerciales) : après C7a.

## Reprise
1. Si la vague 4 est interrompue : lire `docs/wip/b6-*.md`, `b7-*.md`, `c7-*.md` dans les worktrees (`git worktree list`), relancer un agent de reprise par lot inachevé.
2. Fusion de chaque lot : `../gp-tw-merge` (`integration/tw`) : supprimer d'abord les `.import`/`.uid` non suivis sous `game/` (ils bloquent `git merge main` quand main les suit), `git merge main`, `git merge --no-ff <branche>`, clippy + `cargo test`, `core/build.sh`, `--import`, smoke (compter les « smoke OK », 22 actuellement), `git merge --ff-only integration/tw` dans main (si main a bougé : recommencer `git merge main`) + push.
3. Quand C7a colonies est fusionné (`docs/wip/colonies.md`) : lancer C2 (zone de contrôle seule), puis C5 routes commerciales, C4 édits/chaînes.
4. Pistes B6/B7 : minicarte sans haies ni village ; `advance` n'infléchit pas vers un défenseur décalé ; cavalerie qui attend face à un réseau de haies ; carte de piétinement réutilisable pour la boue ; sillage d'écume au gué ; pastilles par corps de bataille.
5. Pistes restantes : pistes musicales dédiées (B3) ; relecture de bataille (replay, L, basse priorité).

## Décisions
- Vague 4 (24/09) : campagne bloquée par colonies → lots sans `ProvinceState` : suite du général (C7, champs `serde(default)`, pas de changement de version de sauvegarde) ; bataille : B6/B7.
- B5 : éléments de site tirés d'un flux dérivé de la graine (batailles existantes inchangées) ; options `--terrain= --season= --village --coast --no-site`.
- C1 : règle de vue `vision.rs` + `data/rules/vision.json` ; brouillard désactivable (Réglages > Carte).
- B3 : musique de bataille = `war.ogg` transformé (filtre, couches sfx), pas de nouvelle piste.
- Piège : le smoke Godot sort en 0 même sur erreur d'analyse ; compter les lignes « smoke OK » (22 attendues depuis C3) et `--import` après un nouveau `class_name`.
- B1 : figures modélisées en Blender scripté (`tools/blender/battle_figures.py`), ADR 0006 ; `--legacy-figures` pour comparer.
- B2 : illustrations d'unités générées par une autre session (non suivies) ; les cartes s'en servent dès qu'elles existent, sinon composition de repli.
- C2 reporté : C4 colonies refond le déplacement.
- P1 : pas de quartier = chef pris tué, −5 piété au vainqueur ; étain déjà utilisé (maison de fonte), schéma inchangé ; `frontier.rs` source unique de la classification frontière.
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lots B1-B4, C1-C6). Thème parchemin conservé, densité TW visée.

## Procédure de fusion
Worktree `../gp-tw-merge` (branche `integration/tw`) puis ff-only dans main + push ; commits avec chemins explicites (index partagé).
