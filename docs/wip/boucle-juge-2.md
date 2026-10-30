# Boucle juge → corrections, 2e campagne (29/10)

Mandat 29/10 : nouveau départ (juges neufs, sans les rapports de la 1re boucle `docs/wip/boucle-juge.md`),
itérations illimitées, arrêt par le joueur ou quand le juge ne trouve plus rien de majeur.
Refontes et révisions d'ADR permises si un juge les justifie. Rien de poussé.
Budget : tout le Meshy restant (145 crédits au 29/10), 2D via Z-Image. Cible 60 fps en Ultra.

Axes : jouabilité, gameplay, visuels, assets, couleur, textures, fps, cohérence, véracité historique et géographique ;
UI/UX, prise en main, équilibrage, accessibilité ; stabilité, chargement/mémoire, aberrations visuelles ;
audio (logs/fichiers), textes/localisation, échelle, cohérence artistique, données ;
authenticité matérielle, plausibilité militaire et politique.

Organisation : 4 juges Sonnet libres par itération (DA, game design, histoire/UX/textes, technique),
rapports `out/juge2/iter-NN-<angle>.md` ; triage par la session principale ; lots de correction en worktrees,
chaque lot vérifie d'abord le constat ; fusion ff-only avec contrôle complet (clippy, tests, smoke), seed bots en fin d'itération.
Ne pas modifier main pendant que les juges tournent (run_bg.sh recompile depuis les sources).

## Itérations
| # | Majeurs | Corrigés | fps Ultra (carte / bataille) | Commit |
|---|---------|----------|------------------------------|--------|
| 1 | Berlin (fichier oublié), arcs longs, cavalerie, économie, provinces qui fondent, damier rose, forêt, mémoire 7-12 Go, mêlée 59 fps | 7 lots fusionnés (DATA, BATDA, SOL, TEXTES, COMBAT 0792, ECO 0793, MEM 0794) + illustration mérinide | mêlée 59 (inchangé), mémoire bataille 6,5 → 3,6 Go, campagne repos 6,2 → 5,4 Go | 59c0047de |

## Meshy
| Date | Asset | Crédits | Reste |
|------|-------|---------|-------|
| 29/10 | village_west, castle_west, town_west, abbey_west (meshy-6 texturé, VIS-R6) | 120 | 25 |

## État / prochaine étape
29/10 : intégration des modifs non commitées de main (sélection des armées plus tolérante), puis itération 1.

## Itération 1 — triage en cours
- « Départ sur Berlin » (DA + histux) : cause = `out/camera.txt` oublié le 28/10 (13.405,52.52), renommé en `out/camera.txt.berlin-2810.bak`. À corriger : ce fichier ne doit agir qu'en build `dev` (le joueur lance depuis le dépôt).
- Coupure internet pendant l'itération 1 : juges design et technique relancés par message.

### Lots de l'itération 1 (lancés 29/10, worktrees)
| Lot | Modèle | Constats | Rapports source |
|-----|--------|----------|-----------------|
| COMBAT | sonnet | arcs longs intouchables, cavalerie écrasante, approche de bataille 123 s | design |
| ECO | sonnet | économie trop riche, provinces qui fondent, banqueroutes, score de règne, attrition hors territoire, bandes seed bots, invariant army_refs | design |
| TEXTES | sonnet | camera.txt hors dev, HUD abrégé, Désordre/Mécontentement, « colonie », Codex futur, Jean le Bel, contrastes, typo, objectifs/prestige/domaine affichés, réserve de recrutement visible | histux, design |
| SOL | sonnet (captures ≤ 5) | damier rose du sol (Angleterre), forêt saturée | art |
| MEM | sonnet | mémoire campagne 7-12 Go, perf_guard.sh zsh, mêlée 59 fps, pics de descente | tech |
| BATDA | sonnet | héraldique cavalerie, bannières minuscules, terrain de bataille plat/horizon, mesh sans normales | art |
| DATA | haiku | valley_courses schéma, ids names_*, couverture des schémas, war_horn, filtre bus lointain | tech |
Reportés à l'itération 2 : tutoriel (recrutement/siège/bataille), illustration d'Abu al-Hasan (Z-Image), Hainaut 1337, date de départ.
ADR : les lots nomment `docs/decisions/XXXX-<lot>-titre.md`, numérotés à la fusion (dernier : 0790).

### Restes pour l'itération 2
- IA de bataille : charge de cavalerie groupée (les chevaliers ne battent pas les arcs longs, ADR 0792), approche encore longue, fixture auto_resolve à régénérer, chevaliers contre hommes d'armes en marche (4 contre 231).
- fps mêlée 59 < 60 : voile volumétrique `step_count` 64 → 40-48, préchauffage des pipelines pendant le voile de chargement (pics de 350-1100 ms).
- Mémoire campagne : 3 Go non comptés par Metal (ADR 0794).
- Prestige de faction non lu (le brancher change l'IA) ; réserve de recrutement à afficher (`game_api::hud::recruit_reserve`) ; seuil de victoire 50 à valider.
- Tutoriel : recrutement, siège, bataille, ravitaillement.
- Bataille : plaques de régiments ennemis en grappe sur l'horizon ; barre du haut tronquée à 1280 px.
- « Edimbourg » cuit dans menu_map.jpg ; Hainaut 1337 ; date de départ.

## Itération 2 (29/10 soir) — virage vers les refontes
Demande du joueur : changements plus importants, analyses plus profondes, parties du jeu révisables.
En cours :
- 4 audits de fond Opus (lecture seule) → `docs/audits/2026-10-29-{campagne,bataille,economie-politique,experience}.md` : diagnostic, 3-6 refontes, specs de lots ;
- juges histoire/UX et technique (iter-02) ; juges DA et design arrêtés (couverts par les audits) ;
- lots IA-CAV, FPS, TUTO (worktrees) ; MEM5 Opus (3 Go de mémoire de campagne).
Suite : choisir les refontes prioritaires dans les audits et les lancer en lots larges.

### Décisions du joueur (29/10)
- Priorité : le visuel, surtout la carte de campagne → audit dédié `docs/audits/2026-10-29-visuel-campagne.md`.
- R6 acceptée : recentrer sur 6-8 maisons jouables approfondies ; les autres factions restent dans le monde, pilotées par l'IA.
- Audit campagne rendu (02292a92f) ; refonte R1 (renommée, actes à dates fixes, ambitions) lancée.
- Audit expérience rendu (df3e688da) : C1 neuf canaux d'attention, C2 guide « boutons », C3 139 factions jouables, C4 bâtir = 130 clics ; refontes XP R1-R6, ordre R4+R1 (XP-POURQUOI, XP-AUDIENCE) puis XP-LISIBLE. À lancer après fusion de TUTO et R1 (fichiers communs) ; machine saturée (attente heavy.sh ≈ 1 h), pas plus de lots d'ici là.
- Audit économie/politique rendu (741e6022a) : la guerre ne coûte presque rien (armée 12-16 % des dépenses), l'ost ne donne pas de soldats, leviers sans interlocuteur, légitimité absente ; 4 défauts de diplomatie. Ordre R0 → R1 (impôt consenti) → R2 (ost et retenue) → R5 → R4 → R3. Lot ECO-R0 (Sonnet) lancé.
- Audit visuel campagne rendu (181f09eda, note 4,8/10) : pas de ciel, lumière froide, sol 360 m/texel + Voronoi gris-bleu, lavis politique qui noie. Refontes R2 bande → R1 ciel/lumière → R4 montagne/neige → R3 sol → R7 végétation → R5 eau → R6 lieux → R8 parchemin. Lots VIS-R2 et VIS-R1 (Sonnet) lancés.
- Choix de mise en scène tranchés en autonomie : météo dégagée au 1er tour (oui) ; aube rose à chaque tour (non, garder le seul crépuscule). Réversibles.
- Meshy prévu : 4 maquettes famille « ouest » (village, château, bourg, abbaye) × 30 = 120 crédits, avec le lot R6 (lieux génériques) ; images sources Z-Image.

## PAUSE (29/10) — levée, reprise en autonomie totale
- Main = 69c48fe9a + notes (MEM5 fusionné, ADR 0795) ; contrôle complet MEM5 lancé (out/check-mem5/status).
- TUTO rebasé, ADR 0796 numéroté, prêt à fusionner (branche worktree-agent-a62be3909ca64d6f4).
- Lots arrêtés en wip dans leurs worktrees : IA-CAV, FPS, HIST4, R1 renommée, BUG-HIST (vérif presque finie), ECO-R0, VIS-R2, VIS-R1.
- Reprise : fusionner TUTO puis BUG-HIST ; relancer les autres lots depuis leur note docs/wip ; XP-POURQUOI/AUDIENCE après TUTO et R1.

## Reprise (29/10 nuit, autonomie totale)
- TUTO (ADR 0796) et BUG-HIST fusionnés ; contrôle complet out/check-tuto.
- Relancés : VIS-R1, VIS-R2, R1 renommée, ECO-R0, HIST4, IA-CAV. FPS gardé pour la fin (mesures A/B illisibles sur machine chargée : à lancer seul).
- VIS-R2 fusionné (ADR 0797) : lavis retiré du cœur, bande au bord ; critère valeur Guyenne/Saintonge −19 % accepté (sol sombre, R1/R3) ; coût GPU à mesurer avec le lot FPS.
- Lancés : XP-LISIBLE, BANC-1, VIS-R7 (végétation), VIS-R6a (images Z-Image des 4 maquettes, hors dépôt ~/dev/cent-ans-raw/vis-r6/, Meshy ensuite). En attente de VIS-R1 : R4 montagne, R3 sol, R5 eau (mêmes shaders).
- ECO-R0 fusionné (diplomatie : trêve en données, truce_started, contrôle or/tribut, Auld Alliance ; sonde eco_series ; 5 métriques, bandes provisoires à recalibrer sur la passe longue). Lancé : ECO-R2a (semonce féodale).
- Meshy : 4 maquettes générées (~/dev/cent-ans-raw/vis-r6/meshy/), intégration VIS-R6b lancée.
- VIS-R1 fusionné (ADR 0799) : dôme dégradé (Atmosphere Bevy écartée : exige Hdr, pas d'échelle), caméra 20-28°, soleil chaud ≥ 1,8 × ciel, brume unique, 1er tour dégagé. Coût GPU illisible (machine chargée) : à refaire avec FPS.
- Nouveau défaut vu après VIS-R1 : plaques des villes lointaines (Édimbourg…) en rang sur l'horizon → correctif PLAQUES-HORIZON lancé.
- Lancés : VIS-R3a (carte de couleur, publication du paquet par la session principale), VIS-R3b (parcellaire typé). En attente : R4 montagne (terrain.wesl, après R3b), R5 eau, R8 parchemin.
- HIST4 fusionné (ADR 0798). BANC-1 prêt (ADR 0800, banc `cargo run --release -p sim-battle --example battle_bench`, 29 s, référence docs/bench/reference-2026-10-29.*). Constats du banc à traiter (IA-PLAN/COHESION) : 27 duels sans contact, 9 sans aucune perte (cavalerie contre infanterie légère n'engage pas) ; « tous à l'attaque » pire que l'IA ; défenseur inactif perd toujours ; Castillon 11/12 et Formigny 1/12 pour l'attaquant de carte à relire contre l'histoire.
- 30/10 : fusionnés BANC-1 (0800), XP-LISIBLE (0801), IA-CAV (0802), VIS-R3a (carte de couleur, sans ADR), VIS-R6b (0803), VIS-R7 (0804) ; main 8323c8cdc, contrôle complet out/check-6lots. Maquettes Meshy copiées dans `game/assets/models/dn/buildings` local ; paquet de modèles non republié (`cent-ans art models-update` à faire).
- Restes : plaque Saint-Denis projetée sur la cathédrale de Paris (occultation par la scène) ; IA-CAV armées mixtes et hommes d'armes à pied balayés ; R7 haies du bocage (→ R3) et saisons printemps/été encore en dur ; R3a routes peu lisibles sur la carte de couleur (trait vectoriel) ; fps/mémoire de R1/R2/R6/R7 à mesurer au lot FPS.
- Lancés : VIS-R5 (eau), IA-PLAN-1 (planificateur), BANC-2 (auto-résolution). En cours : R1 renommée, VIS-R3b, PLAQUES-HORIZON, ECO-R2a.
- 30/10 : test de schéma battle_bench ajouté (9adcb4260) ; fusionnés PLAQUES-HORIZON (plaques lointaines qui s'effacent selon la distance, sans ADR) et R1 renommée (0805, seuil de victoire 43) ; titres d'ADR 0795-0799 numérotés. Contrôle out/check-r1. Passe longue seed bots de R1 encore dans son worktree (agent-adc1d3f985235a4e2, à garder jusqu'au rapport).
- Lancés : XP-POURQUOI (causes), R1-UI (renommée et ambitions à l'écran). Reste : plaques de bataille lointaines (lot à faire).
- 30/10 nuit : fusionnés VIS-R5 eau (0806 ; rivières toujours invisibles à d 600, berges non vues), BANC-2 (0807, accord auto/3D 73 → 81 %, `data_hash`), ECO-R2a semonce (0808), IA-PLAN-1 (0809, drapeau `planner` défaut legacy ; Crécy/Castillon se dégradent sous plans → IA-PLAN-2), R1-UI (panneau d'ambitions, seuil 43). Main rouge après R1 (2 tests de campagne, fixture, codex) → lot VERT lancé.
- Points ouverts R1-UI : aucune invite au tour 1 pour choisir les ambitions (→ XP-AUDIENCE) ; boutons figés non grisés ; renommée absente du HUD.
- 30/10 : VERT fusionné (test d'arc : saut d'années tuait Édouard III ; ambitions en2_amb_guyenne/bu3_amb_arras renforcées ; fixture régénérée, accord 80,8 % juste au seuil ; codex). XP-POURQUOI (0810 : causes de variation, Récit de saison, tendances de la barre) et RIVIERES (0811 : alpha nul du ruban élargi dans river.wesl ; Seine visible à d 600) fusionnés. Contrôle out/check-vert.
- Lancés : ECO-R1ab (impôt consenti), XP-AUDIENCE (Audience de saison + invite d'ambitions). Restes : fleuves pâles à d 900, bande riparienne illisible ; CLI codex-bundle cherche tools/data/ depuis un worktree ; accord auto/3D à 80,8 % (marge mince).
- 30/10 : main vert (out/check-vert). Passe longue seed bots sur main (out/seedbots-long-main-2026-10-30.md) : régression `ai_refused_orders` 430-515/partie (bande 0-40) → lot IA-REFUS lancé. Dérives d'équilibre à présenter au joueur (non inscrites dans seed-bots-findings.md) : reign_score sous [6 ; 40] pour 5 bots (échelle R1 : passif = 0, bande à recalibrer ?), army_spend_share 0,46-0,56 (au-dessus de la bande, effet voulu de la semonce/ECO ?), revoltes/prises rebelles aggressor-sailor, banqueroutes turtle/random, factions_eliminated 30 (builder). Rapport passe longue R1 : out/seedbots-long-r1-2026-10-30.md.

## 30/10 matin
- Fusionnés : ECO-R1ab (ADR 0813, ambition `fr2_amb_compagnies` réajustée, glossaire), IA-REFUS (refus IA 430-515 -> 23-38/partie, sans ADR).
- En cours : VIS-R3b, ECO-R1c, ECO-R1d, IA-PLAN-2, récupération MEM3 + FPS mêlée (branches à rebaser, non fusionnées).
- Ensuite : VIS-R4 après R3b ; COHESION-1 après IA-PLAN-2 ; VIS-R8.

## Passation 30/10 (nouvelle session headless)
Mandat : autonomie totale, boucle juge 2 illimitée (visuel campagne prioritaire, 60 fps Ultra, R6 6-8 maisons). Ne jamais finir un tour « en attente » : tant que des lots tournent, attendre par Monitor/boucle until, puis fusionner et relancer.

Lots lancés par la session précédente (worktrees `.claude/worktrees/agent-<id>`, branche `worktree-agent-<id>`) ; fini = worktree plus `locked` dans `git worktree list` et note wip « terminé » :
- a771f004f6088afb4 VIS-R3b parcellaire typé (ADR à numéroter)
- aa5182d13ee2ab5e2 ECO-R1c IA et Angleterre (note docs/wip/eco-r1c.md)
- a81b1ae605a828a40 IA-PLAN-2 (note docs/wip/ia-plan-2.md)
- a5dbb10991dc43fa8 récupération MEM3 (agent-a0377ad7d1eea36f9) + FPS mêlée (agent-a7fa6dbbe9cdf1f9d) : rebase, ne fusionne pas
- af539704f9ee95346 ECO-R1d : TERMINÉ, prêt (smoke complet à relancer), sans ADR propre (0813)
Prochain ADR libre : 0814. Script de fusion : tools ou scratchpad `merge_lot.sh` (rebase main, numérote l'ADR, ff-merge) — à recréer dans tools/ si absent.

Économie de jetons (appliquer) :
1. Sous-agents Haiku par défaut, Sonnet seulement pour les lots durs (IA bataille, économie) ; jamais Opus.
2. Fusions par un agent de fusion dédié (Sonnet) par lot de 2-4 branches, pas par la session principale.
3. Réserver le numéro d'ADR au lancement ; sérialiser les lots qui touchent la même zone (IA-PLAN-2 puis COHESION ; VIS-R3b puis VIS-R4).
4. Rapport d'agent ≤ 15 lignes, détails dans docs/wip/<lot>.md.
5. Dans les agents : seed bots --quick seulement ; une passe longue sur main par lot de fusions.
6. Un check complet (tools/full_check.sh + smoke + pytest + seed quick) par lot de fusions, pas par lot.
7. 3-4 agents en parallèle au plus.
Suivants : VIS-R4 (après R3b), COHESION-1 (après IA-PLAN-2), VIS-R8 parchemin, R2-b lettres de retenue, textes XP encore codés, lot FPS seul sur machine calme.
Constats seed bots longs : présentés au joueur, il choisit (ne rien écrire dans seed-bots-findings.md sans lui).

## Session headless 30/10 (game-project-8e)
- ECO-R1d fusionné (619015b68, sans ADR) ; contrôle complet + smoke + pytest : out/check-r1d.
- Lots encore actifs (session précédente) : VIS-R3b, ECO-R1c, IA-PLAN-2, FPS/MEM3 ; surveillés par out/watch_lots.sh.

## Reprise (30/10, session headless arrêtée)
- main = 5fdd09a1a, propre. ECO-R1d fusionné (worktree agent-af539704f9ee95346 à supprimer, fusionné).
- Check en cours au moment de l'arrêt : out/check-refus/status (+ passe longue seed bots, rapport dans core/target/seed-bots/ le plus récent).
- Branches d'agents (les agents de la session précédente peuvent finir seuls ; fini = plus « locked ») : VIS-R3b a771f004f6088afb4, ECO-R1c aa5182d13ee2ab5e2, IA-PLAN-2 a81b1ae605a828a40, MEM3 a0377ad7d1eea36f9 + FPS mêlée a7fa6dbbe9cdf1f9d (rebasées par un agent de récupération, à vérifier puis fusionner).
- Objectifs détaillés et règles : docs/prompt-session.md.

## Session 30/10 matin (7 h) — vague 3
- Agents de la session précédente morts (verrous périmés levés). Worktrees MEM3/FPS gardés pour une fenêtre machine calme (lot seul).
- Lancés (Sonnet, effort medium) : VIS-R3b fin (agent-a771f004f6088afb4, ADR 0814), ECO-R1c fin (agent-aa5182d13ee2ab5e2, 0815),
  IA-PLAN-2 (agent-a81b1ae605a828a40, 0816), CAMP-R6 8 maisons jouables (0817), XP-R3a livres Royaume + Chronique (0818),
  PLAQUES-2 Saint-Denis + plaques de bataille lointaines (0819 si besoin). Prochain ADR libre : 0820.
- Ensuite : agent de fusion, check complet, juge visuel Opus frais, puis VIS-R4 (après R3b), COHESION-1 (après IA-PLAN-2), VIS-R8, R2-b.

## ARRÊT demandé par le joueur (30/10)
- Tout arrêté. main = 60e6d7c82 : VIS-R3b fusionné (ADR 0814 ; critères teinte 3/4 non atteints, à juger). Check complet sur main NON lancé.
- Prêts, non fusionnés : CAMP-R6 (agent-a65d62579f5332956, 0817), IA-PLAN-2 (agent-a81b1ae605a828a40, 0816, défaut legacy),
  PLAQUES-2 (agent-a3bc6a6b1ae8c9817 ; fmt HUD non commité dans le worktree), XP-R3a (agent-a956e1bb95065cfe9, 0818),
  CAMP-R6b (agent-a3ef7512eced87a7a), ECO-R1c (agent-aa5182d13ee2ab5e2, 0815 ; armée IA 27-37 %, sous la bande).
- À corriger sur main : snapshot faction_assembly (test client), fmt HUD (audience, feed, story, text).
- Suivants : IA-PLAN-2b (patience du défenseur, Castillon), R2-b retenue, XP-R3b, VIS-R4, juge visuel Opus. Prochain ADR libre : 0820.

## Reprise 30/10 (9 h 20, autonomie totale, docs/prompt-session.md)
- Worktree VIS-R3b supprimé (fusionné).
- Lancés : FUSION-A (agent de fusion Sonnet : ECO-R1c 0815, IA-PLAN-2 0816, PLAQUES-2 0819, XP-R3a 0818, CAMP-R6 0817, CAMP-R6b ;
  correctifs main snapshot faction_assembly + fmt HUD ; un check complet) ; VIS-R4 montagne étagée et neige (ADR 0820).
- Après FUSION-A : juge visuel Opus frais, IA-PLAN-2b (0821), R2-b retenue (0822), XP-R3b (0823). Puis COHESION-1, VIS-R8. Prochain ADR libre : 0824.
- 9 h 45 : lancés depuis fusion-a (avant son ff) : IA-PLAN-2b (0821), ECO-R2b retenue (0822), XP-R3b livres (0823 si besoin),
  juges frais itér. 3 : visuel (Opus), technique, jeu (campagne/éco/XP) → docs/audits/2026-10-30-juge-*-iter3.md. Prochain ADR libre : 0824.
- Plafond joueur : arrêt à 200 $ d'équivalent API pour la session (tools/session_cost.py <session-id>, journal out/session-cost.log ;
  veille qui sonne à 190 $). 6,6 $ à 9 h 45.
- 10 h 50 : FUSION-A fusionnée (737f049e9 ; 6 lots, ADR 0815-0818 ; check vert sauf `real_table_and_glbs` : glb ignoré absent du worktree, revérifié sur main dans out/check-main-postA ; passe longue seed bots sur main lancée au même endroit).
- Juges itér. 3 rendus : technique (docs/audits/2026-10-30-juge-technique-iter3.md, lots L-A..L-F), visuel (…-juge-visuel-iter3.md, 4,5/10, lots L1-L6).
- VIS-R4 fini (branche worktree-agent-a47dbd6c148e66466, ADR 0820), en attente de la fusion d'itération.
- Lancés : VIS-L2 habitat (0824), VIS-L6 bataille (0825), VIS-L1 sol-photo (0826, part de la branche VIS-R4). Prochain ADR libre : 0827.
- Règles changées par le joueur : sondes seulement à la fusion (tools/checks_for_diff.sh), fusion de toute l'itération sauf lot en retard. 27,9 $ à 10 h 45.
- À lancer ensuite : L-A compteurs de perf, L-C prévision asynchrone ; L4 eau après L1 ; L3/L5 après les plaques ; lots du juge de jeu.
- 30/10 : ÉCONOMIE EN PAUSE (décision du joueur) : ni juge ni nouveau lot économique ; ECO-R2b (ADR 0822) est mené à terme et fusionné avec l'itération, c'est le dernier. Exception sondes accordée : battle_bench dans les lots d'IA de bataille (eco_series idem, sans objet pendant la pause). Permissions rm autorisées (settings.local.json).
- 30/10 ~10:20 : audit juge jeu iter3 dans main (6855cba7c ; lots A CAMP-V, B CAMP-S, C CAMP-R2, D XP-SOBRE, E CAMP-GEN, F PILOTE-JUGE ; ordre D → E → A → B après XP-R3b → C après IA-PLAN-2b). Lancés : D XP-SOBRE (ADR 0830), E CAMP-GEN (0831), IA-PLAN-2c+COHESION-1 (0828, part de la branche IA-PLAN-2b), L-F DATA-GARDE (0829). Prêts à fusionner : VIS-R4, IA-PLAN-2b (via 2c), XP-R3b (007c44b4d, smoke books à lancer). Pilote du juge non commité dans son worktree (agent-a7268a3b37806d3a4 : examples/judge_play.rs) → lot F. Prochain ADR libre : 0832.
- Prêts à fusionner (suite) : CAMP-GEN (63b9a3a56, ADR 0831), VIS-L6 (fc6c5c8b8, sans ADR ; sol tenu s 0,46/v 0,48, chevaliers s ≥ 0,25 NON vérifié → juge visuel après fusion). Lancés : CAMP-V (0832, part de CAMP-GEN), VIS-L3 air/horizon (0833 si besoin). L5 plaques attend la fusion (markers/hud), L4 eau attend L1. Prochain ADR libre : 0834.
- Prêts (suite) : DATA-GARDE (ADR 0829 ; 2 495/2 495 fichiers couverts, manifeste, chargement 36 → 22 ms ; client hors périmètre), VIS-L2 (ca20bf19d, ADR 0824 ; shapes.rs supprimé, noyau de hameau en décalque à texture générée par code depuis palettes → à juger, potentiellement contraire à « pas de code-made » ; maquette village_west non posée ; rendu non capturé). Lancé : MEM6 phase 1 (part de MEM3, ADR 0786, hors fichiers de sol). Pilote du juge sauvé dans out/judge-pilot/ (lot F après A et E).
- XP-SOBRE prêt (4c666bf22, ADR 0830 ; rappels répétés ≤ 3/type/10 saisons ; jalons de règne à recalibrer ; bouton « Rappeler » non câblé). FUSION-B lancée (agent Sonnet, worktree fusion-b, verrou out/merge.lock) : VIS-R4, DATA-GARDE, CAMP-GEN, XP-SOBRE, XP-R3b, VIS-L6, VIS-L2. Après : juge visuel Opus frais, CAMP-S (B) sur la Chronique, VIS-L5 plaques.
- Passe longue seed bots sur main post-A (out/check-main-postA/seed-long.log) : dérives d'équilibre à présenter au joueur (non inscrites) : reign_score 1-3,5 (bande 6-40) pour builder/chaos/random/sailor/turtle ; aggressor trésor −4 471, troubles 33, soulèvements 13 ; random-legal banqueroutes 28, révoltes 11 ; sailor/random provinces finales ≤ 1, prises rebelles 4-6.
- VIS-L1 prêt (fd24daaa6, ADR 0826) : terrain.wesl 2 971 → 364 lignes (11 modules), plaine s 0,21 → 0,45, couture Normandie 12° → 1,8°, peigné ÷3 ; non tenu : v forêt/pré ≤ 0,6 (0,63-0,68) ; pas de clipmap ni Z-Image ; coût GPU non mesuré. Ajouté à FUSION-B. L4 EAU débloqué après FUSION-B.
- PERF-COMPTEURS prêt (02106776f, ADR 0827) : objet `work` par phase (draws, triangles, pipelines, CPU fil principal), garde perf sans temps mural dans full_check. Références : carte d30 430 draws/1,69 M tri ; mêlée 441 draws/6,34 M tri ; empreinte carte 6 942 Mo, bataille 3 494 Mo. Ajouté en dernier à FUSION-B avec recalibrage des seuils. MEM3/FPS à rebaser dessus.
- VIS-L3 prêt (8d7e20afb, ADR 0833) : brume unique (passe post, terre = mer), nuages procéduraux, bande de brume ≈ 10 % à d 60 ; NON tenus : ciel s 0,17-0,24 (cible ≥ 0,25), étiquettes/cavaliers au-dessus de l'horizon (→ VIS-L5 : markers/armies lisent Atmosphere.fog, DepthFog::amount ≥ 0,6). Ajouté à FUSION-B avant PERF.
- 30/10 ~11:05 : CAMP-V prêt (f1222ccce, ADR 0832 ; 47 verbes de maison 5-8 par maison, leviers de départ ouverts en 1337, pression si inemployés ; écarts : ≥ 2 actions/saison non tenu en 4e saison pour 4 maisons, score de règne ≥ 8 non mesuré, pas de panneau client). Paquet FUSION-C (après FUSION-B) : CAMP-V, IA-PLAN-2c, MEM6 p1, ECO-R2b. ECO-R2b : agent arrêté en route (dernier commit 193ff486e, ADR non commité), relancé pour finir + eco_series.
- Lancé : BUILD-DX-a (ADR 0834 ; gc_targets.sh, heavy.sh FIFO, profil release-fast ; découpage du client reporté quand peu de lots en vol).
- IA-PLAN-2c + COHESION-1 prêt (68fe31358, ADR 0828) : 6/6 issues historiques (Formigny 11/12, Castillon 0/12), accord auto/3D 84,6 % ; régressions fr_en_hills 6→1, great_battle 3→0 ; HdA à pied 0/6 contre charge ; planner reste legacy. Lancé COHESION-2 (ADR 0835) sur cette branche : tir en 4 termes, HdA tiennent la charge, rééquilibrage. FUSION-C prendra COHESION-2 (contient 2b/2c) s'il est prêt, sinon 2c.
- BUILD-DX-a prêt (f82ec0b1e, ADR 0834) : gc_targets.sh (0 Go libérable aujourd'hui, à blanc), heavy.sh FIFO testé, release-fast. Pour FUSION-C.
- ECO-R2b fini (618dcc110, ADR 0822) : part de l'armée France 32-40 % (bande 50-70 % NON tenue, avant 26-37 %), plus grosse armée ≈ 4 k (bande 8-20 k NON tenue ; cause probable : plafond de taille d'armée, hors lot) ; pas de banqueroute France. Économie en pause : accepté tel quel, écarts consignés pour la reprise éco. Pour FUSION-C.
- MEM6 p1 prêt (29b32015c, ADR 0786 complété) : bataille stabilisée 5 880 → 4 840 Mo (cible 4 500 non tenue), campagne au repos hors Metal 2 950 (≤ 3 000 tenu), banc carte +1 830 (critère +1 000 non tenu, caches sol/détail → phase 2), retour campagne +1,1 Go (décharge symétrique → phase 2). Pour FUSION-C.
- 11:45 disque 12 Go → 128 Go libres : target/ supprimés des 14 worktrees de lots finis (branches gardées).
- 30/10 ~12:30 : FUSION-B dans main (74e868849) : 10 lots, check complet vert, smoke 15/15, seed quick 0 constat, seuils perf recalibrés (mêlée triangles 6,34 M → 3,17 M inexpliqué → juge visuel). Lancés : FUSION-C (BUILD-DX-a, MEM6 p1, ECO-R2b, CAMP-V), juge visuel Opus iter4 (→ docs/audits/2026-10-30-juge-visuel-iter4.md), VIS-L4 eau (0836), VIS-L5 plaques/parchemin + masquage au-delà de l'horizon (0837), CAMP-S crises à horloge (0838), FORECAST-ASYNC (0839). Passe longue seed bots sur main post-B → out/seed-long-postB/. En attente de FUSION-C : CAMP-R2 (lot C), PILOTE-JUGE (lot F), DATA-GARDE client, MEM6 p2, FPS-MÊLÉE.
- Passe longue seed bots post-B (exit 0) : mêmes dérives que post-A, valeurs identiques (FUSION-B ne touche pas la sim) : reign_score 0,75-3,5 (bande 6-40) pour tous les bots ; aggressor trésor −4 471, 29,5 banqueroutes, 20,5 révoltes, 0,75 province ; random-legal 28 banqueroutes ; sailor/random ≤ 1 province, prises rebelles 4-7. À présenter au joueur (rien dans seed-bots-findings.md).
- VIS-L4 partiel (05d74e900) : Loire ruban ≥ 10 px s 0,35 (tenu), Manche s 0,77 v 0,27 (tenu), Gironde redessinée mais liseré pâle (écume au loin) ; chaîne carte non rejouée. Lancé VIS-L4b sur cette branche (écume selon la largeur, Atlantique, splat/colormap/provinces). FUSION-C : test IA général d'armée cassé par ECO-R2b → consigne : lever avant de nommer les généraux.
- Juge visuel iter4 (d5d5388ba) : 5/10 (4,5 avant). G1 : saturation ×2,8 globale (parchemin fluo, neige jaune), forêts plus claires que prés, sol en aplats + prismes de champ low poly, étiquettes dans le ciel, postérisation en bataille après ~2 300 images ; hamlet_core = texture codée (interdit) ; triangles mêlée = artefact du banc (arrêt avant contact). Lancés : M1 chroma (0840), M5a bataille stable + banc (0841), M3 parcellaire peint + décalque hameau Z-Image (0842). VIS-L5 prévenu (étiquettes de régions, ne pas régler contre la saturation). À venir : M2 photo aérienne régionale (après M1, L4b), M4 échelle des armées + ciel (après L5), M6 parchemin qui monte / heure du jour.
- 30/10 ~13:10 : plafond de coût relevé par le joueur à 300 $ (au lieu de 200) ; veilleur de coût à 290 $.
- FORECAST-ASYNC prêt (7a904f4db, ADR 0839) : prévision + récit sur AsyncComputeTaskPool, fil principal 48,7 ms (p95 76,6) → copie d'état 0,63 ms ; test d'égalité vert ; incrémental non fait (modules en chantier) ; banc client release avant/après à faire à la fusion. Pour FUSION-D.
- VIS-L5 partiel (da36cbcb6, ADR 0837) : masquage par la brume (lieux, régions, fleuves, plaques et cavaliers d'armée) + zone sûre HUD + écu de faction sur la plaque. Non fait : critères parchemin (parchment.wesl, après M1), Saint-Denis, Alpes d 120 / Paris d 25 non vérifiés, figures fantômes dans le ciel ; masquage trop strict (2 noms visibles sur 405 à Paris d 60, contre 8 avant) → à régler. Suite : lot VIS-L5b + M4 après M1. Pour FUSION-D.
- 30/10 ~14:00 : joueur « il faut des changements plus ambitieux ». Lancés trois refontes larges : M2 photo aérienne régionale depuis Sentinel-2 (0843), CAMP-R2 la guerre en saisons + première campagne datée 1337-1340 (0844), BAT-R2R5 trois batailles par armée + captifs et rançons (0845). Règle pour la suite : chaque lot = refonte d'un système avec critères mesurables, pas de retouche de réglages ; les reprises partielles sont fusionnées dans des refontes plutôt que relancées seules. Prochain ADR libre : 0846.
- VIS-L4b partiel (7b9ffa15b) : écume selon la largeur (sea.wesl, sans sampler en plus), Gironde s 0,52 v 0,44 (tenu) mais h 177 (cible 195-215, teinte de hauts-fonds) ; Atlantique v 0,32 (cible ≤ 0,28) ; splat dérive sur 13,9 M px (non commité), geo provinces KeyError prov_alencon ; ATTENTION pyramide partagée (symlink) déjà modifiée par la Gironde de L4. Pour FUSION-D avec L4.
- CAMP-S prêt (c95e3ca7e, ADR 0838) : 14 dossiers de crise à horloge (Cadzand, Artevelde, laine, Dunbar, Bretagne 1341, légat, États, soldats sans solde → routiers, Grandes Compagnies, murmures, peste), défaut qui coûte ≥ 8 points de plus que le meilleur choix, plafonds 2 ouverts / 1 par saison ; joueur seulement ; section dans le livre Chronique. Non fait : mission échouée sous gage d'allié (pas de notion de gage, reporté). Conflit attendu sur l'enum Order avec CAMP-V/ECO-R2b. Pour FUSION-D.
