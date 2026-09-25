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
| 4 | B6 IA tactique qui exploite le site (haies, village) + libellé du terrain avant-bataille ; B7 finitions visuelles (fanions de lance, repères regroupés au loin, neige, éclaboussures de gué) ; C7 suite du général à la Medieval II + année de décès + mise en page fiche < 1500 px | B7 **fusionné** (7441f5c) ; B6 **fusionné** (e618e41) ; C7 **fusionné** (12a53bc) |
| 5 | C6 agents Medieval II (espion, émissaire, prédicateur) ; C2 zone de contrôle après C7a colonies | C6 **fusionné** (12a53bc, ADR 0009) ; C2 relancé en vague 6 |
| 6 | C2 zone de contrôle (**suspendu** : remplacé par M2 mouvement libre d'une autre session, `docs/design/2026-09-24-mouvement-libre.md`, ZdC 8 km sur grille ; reprendre après M2 pour effets diplomatiques/UI non couverts) ; C5 routes commerciales + accords ; C4 édits régionaux + chaînes de bâtiments ; B8 suites B6/B7 (IA attaquant/bocage/poursuite, minicarte de site, boue, écume de gué) | **en cours** (4 agents, notes `docs/wip/c2-zone-controle.md`, `c5-commerce.md`, `c4-edits-chaines.md`, `b8-suites-bataille.md` dans leurs branches) |

## État courant (25/09, passation à l'orchestrateur de nuit)

Fusionnés par cette session : P1, P2, B1-B7, B8 (IA), B8b (visuels, b420bd6), C1, C3, C6, C7.
**C4 et C5 fusionnés par l'orchestrateur de nuit** (92ed8a4c ; accord commercial unifié avec les traités DP1). **Vague 6 TW close** ; pistes d'équilibre transmises à EQ1 (nuit).
Travail d'intégration transmis : `integration/tw` (6ebe9658) = C5 + C4 par-dessus M2, vérifié (58 suites, smoke 23 OK dont trade et edicts) ; commits utiles 321222ae (clippy c5_trade) et 7d754085 (C5 adapté à `ArmyPosition`). Après M3 : `resolve_trade` dans `resolve_end_of_turn`, `ai_choose_edicts` dans `plan_turn`.
C2 zone de contrôle : couvert en grande partie par M2 (ZdC 8 km) ; reste éventuel l'affichage (`docs/wip/c2-zone-controle.md`).
Main a fortement évolué pendant la nuit (M2/M3 mouvement libre, G1, UI2, V2-V4, BV1/BV2, NV1 batailles navales…) : relire `docs/wip/nuit.md` et `docs/wip/mouvement-libre.md` avant toute nouvelle vague TW.

## Reprise
1. Si la vague 4 est interrompue : lire `docs/wip/b6-*.md`, `b7-*.md`, `c7-*.md` dans les worktrees (`git worktree list`), relancer un agent de reprise par lot inachevé.
2. Fusion de chaque lot : `../gp-tw-merge` (`integration/tw`) : supprimer d'abord les `.import`/`.uid` non suivis sous `game/` (ils bloquent `git merge main` quand main les suit), `git merge main`, `git merge --no-ff <branche>`, clippy + `cargo test`, `core/build.sh`, `--import`, smoke (compter les « smoke OK », 22 actuellement), `git merge --ff-only integration/tw` dans main (si main a bougé : recommencer `git merge main`) + push.
3. Mouvement libre (autre session, M2/M3, `docs/wip/mouvement-libre.md`) réécrit `movement.rs` et `end_turn`, STATE_VERSION 6 : fusionner C4/C5/B8 tôt ; C6 agents (Dijkstra sur les fonctions de `movement.rs`) sera adapté par M2. Fusions de la vague 6 : C4 et C5 touchent tous deux `economy.rs` (conflits probables), C2 `movement.rs`. main bouge souvent (autres sessions) : fusionner main dans `integration/tw` juste avant le ff ; si seuls des docs ont changé, pas de nouvelle vérification.
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
