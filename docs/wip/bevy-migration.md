# Migration vers Bevy (branche `bevy-migration`, worktree `../gp-bevy`)

Mandat (14/10 au soir) : nuit en autonomie complète, largeur maximale avec de nombreux agents Sonnet
contrôlés par Opus, **ni fusion dans `main` ni push**. Verdict : ADR 0347 ; architecture : ADR 0357.

## REPRISE — pause du 16/10 ~12 h (demandée par le joueur)
- Budget joueur (16/10) : 20 agents Sonnet + 10 Opus en parallèle au plus ; des agents Opus fusionnent par fournées.
- Outils : recopier `docs/wip/bm-outils/*` dans le scratchpad de la session (le dossier temporaire a déjà été
  effacé une fois) ; corriger le chemin `S=` de merge_brief.md ; `wave8.md` = ajouts de consigne de la vague 8 ;
  `m8check.sh` (clippy puis tests, codes dans <préfixe>.rc) et `m8rm.sh` (nettoyage contrôlé) pour bm/merge8.
- **showfix** (Opus, travaille DIRECTEMENT dans gp-bevy, commits 96a1fce94, 5c46a51f0, f915a9f3c non compilé) :
  note `docs/wip/bm-showfix.md`. Le « 2/255 » était un faux problème (comparaison de deux captures déjà
  post-fusion ; contre la réf. de 08 h 24 l'écart est 40-74/255). Rivières enterrées de 10-25 m à d=60, cause
  racine non trouvée (prochaine étape précise dans la note) ; toits coniques des tours de siège corrigés, non
  vérifiés ; ciel de bataille blanc et arbres proches pâles non traités ; recaptures non faites.
- **Vague 8** (13 lots Sonnet, tous FINIS, `.bm-ready`, worktrees `../gp-bm-<lot>` intacts) :
  setbattle (FUSIONNÉ dans bm/merge8, d04fd675e), diplo6, fxsmoke, naval2, bhud6, uisound2, setcamp, flowinteg,
  uileft2, uiimg2 (touche core sim-campaign), access, econ2, council2. Rapports résumés dans les notes bm-<lot>.md.
- Fusion : branche intermédiaire `bm/merge8` (worktree `../gp-bm-merge8`, target chaud) pour ne pas gêner showfix
  dans gp-bevy. Reprendre : un agent de fusion Opus fusionne les 12 lots restants dans merge8 (fournées de 4),
  puis `bm/merge8` → `bevy-migration` quand showfix a fini. Conflits attendus : advisor.rs (bhud6/setbattle),
  audio/battle.rs (uisound2/setbattle), armies/mod.rs (fxsmoke/setcamp), panels/diplomacy/mod.rs
  (diplo6/uisound2), hud/minimap.rs (econ2/uileft2).
- Ensuite : vague visuelle (végétation, parchemin, villes, eau, ciel, bvis4…) selon showfix ; restes listés dans
  bm-restes.md et dans chaque bm-<lot>.md ; puis recaptures 8 scènes → écarts au joueur → banc calme → pilote →
  bascule. Jamais de push. Machine très chargée pendant la vague (charge ~110, clippy à froid 19 min) : 13 lots
  en parallèle, c'est la limite pratique.

## (ancienne) REPRISE — pause du 15/10 ~15 h (limite hebdomadaire Claude Code)
- Tout est fusionné dans `bevy-migration` (HEAD `ac91341f1` + ce commit), aucun worktree
  `gp-bm-*`, aucun agent ni verrou. 1029 tests verts, smoke vert (muet). Jamais poussé.
- Vague « parité visuelle » finie : campvis2-4, bvis1-3, vegvis, weather4, water3, campwater,
  borders2, towns2, mats, siegemat, ga3, fg3, bherald, aflags, lifeanim, fxfire, uiimg,
  uisound, uileft, hudfix, council, svgconv, dataassets, devtools1-2, launcher, nofocus.
- **Problème ouvert n° 1 (à reprendre en premier)** : les captures de référence après toutes
  ces fusions (`tools/compare_views.sh --side bevy`, release, hors écran, réglages par défaut)
  ne bougent presque pas par rapport à avant (écart moyen 2/255 en bataille, 3 au siège,
  9 à camp_0060, 1,4 à camp_2600). Ciel de bataille blanc, arbres proches = pastilles pâles,
  rivières presque invisibles à d=60, toits des tours de siège plats au lieu de cônes sombres.
  Les lots disaient le contraire → leurs effets n'atteignent pas ces scènes. Pistes :
  préréglage de qualité par défaut, rendu `--offscreen` (HDR/skybox/prépassage/SSAO),
  drapeaux d'env ou de données coupés (CENT_ANS_VEG_MODELS, halo coupé), chargements
  asynchrones (paquets 2048 = 8-11 s, cube HDRI) non finis à l'image capturée (campagne
  à 600 images, bataille à 300), caméras différentes des mesures des lots. Un agent « showfix »
  avait été lancé puis arrêté avant tout commit.
- Ensuite : recaptures des 8 scènes → écarts défavorables à Bevy présentés au joueur → vagues de
  correctifs → banc machine calme → partie pilote → bascule (lanceur Bevy Windows/Linux/macOS,
  retrait du pont Godot, fusion dans `main` sans push).
- Restes par lot : `docs/wip/bm-restes.md`. Outils d'orchestration (consigne commune des
  agents, consigne de fusion à verrou `mkdir`, création/suppression de worktree, garde-disque,
  journal des fusions) : `docs/wip/bm-outils/` — les recopier dans le scratchpad de la session.
- Règles toujours valables : jamais de focus ni de son (Bevy `client/run_bg.sh`, muet + hors
  écran par défaut) ; réutiliser les assets du joueur ; `cargo test` SANS `CENT_ANS_MUTE`
  (le test audio échoue sinon) ; sccache (`source client/sccache-env.sh`) ; pas de clone de
  `target` entre worktrees ; un agent de fusion Opus neuf par fournée de 2-4 lots.
- Si la reprise se fait via OpenRouter (facturé au token) : agents de lot en Sonnet, 2-3 à la
  fois au plus.

## État
- [x] V0 squelette : `client/` (plugins vides enregistrés, états, surface agent, chargement
      sim), `core/crates/game-api` (`faction_summary` + tests). Commits `ceb42d3ac`..`decfefa38`.
- [ ] V1 (parallèle, 9 lots lancés ~2 h) : ui/kit, campaign/terrain+camera, campaign/provinces,
      campaign/water, campaign/markers, campaign/armies, battle/scene+units, audio, game-api
      (vues HUD/province/faction/armée). Worktrees `../gp-bm-<lot>`, branches `bm/<lot>`.
- [ ] V2 : menus (accueil, faction, réglages, sauvegardes), HUD campagne + fin de tour,
      fiche de province, armées (sélection, ordres, chemin), diplomatie, technologies,
      bataille (ordres, HUD, avant-bataille, résultat), codex, végétation/forêts.
- [ ] V3 : le reste (vie de la carte, sièges, naval auto-résolu, conseil, féodalité, chronique…).

## Règles des lots
- Un lot = un dossier de `client/src/` (ou un module de `game-api`) ; ne toucher ni `main.rs` ni
  les dossiers des autres.
- Branche `bm/<lot>` depuis `bevy-migration`, worktree `../gp-bm-<lot>`, `client/target` cloné
  (`cp -c -R`, APFS) depuis `../gp-bevy/client/target`.
- Référence : le GDScript équivalent dans `game/scripts/…` (même dépôt).
- Vérification : `cargo clippy --features dev -- -D warnings`, `cargo test`, passe
  `--dump-state` ; pas de capture pour les agents Sonnet.

## Journal
- 15/10 ~1 h 45 : banc n° 4 valide, verdict Bevy (ADR 0347). Branche créée depuis `proto-bevy`
  (qui contient `main` du 15/10, `ef2bbfaae`).
- 15/10 ~2 h : squelette V0 commité (menu → campagne vérifié par vidage). Vague 1 lancée
  (9 agents : uikit, terrain et battle en Opus, les 6 autres en Sonnet).
- 15/10 ~2 h 15 : 7 lots V1 fusionnés (armies, gameapi, audio, provinces, markers, water, uikit),
  worktrees supprimés ; vidage Campaign combiné OK (13 armées, 400 villes, 213 provinces, eau,
  audio). Galerie du kit vue : bon rendu, marges intérieures manquantes (lot kitfix).
  Restes de chaque lot : `docs/wip/bm-<lot>.md`.
- Vague 2 lancée (8 lots Sonnet) : kitfix, menu (accueil, faction, sauvegardes, pause), settings,
  hud (barre, alertes, fin de tour), panels/{province,diplomacy,tech,army}. Terrain et battle (V1)
  encore en cours. Attention disque : `bm_wt.sh` clone désormais assets et data en APFS.
- 15/10 ~3 h : `run_bg.sh` transmet les variables `CENT_ANS_*` à l'application (`open --env`).
  Disque : `target` du prototype supprimé (11 Go, reconstructible par `cargo build` dans
  `tools/proto_bevy/bevy`) ; chaque worktree de lot diverge d'environ 1 à 4 Go de son clone.
- 15/10 ~3 h 15 : vague 2 fusionnée (settings, hud, pprovince, ptech, parmy, pdiplo, menu, kitfix).
  Vague 3 lancée : 20 agents (Opus : look, orders, bunits, flow, vegetation ; Sonnet : markers,
  armies, life, weather, bhud, bdecor, codex, siege, court, feudal, chronicle, camera, hud2,
  province2, parmy2). En attente d'une place (limite du harnais : 20 agents simultanés) :
  faction, menu2. Ensuite : lot drape (frontières/eau drapées via relief.wesl) après look.
- 15/10 ~3 h 40 : vague 3 entièrement fusionnée (22 lots ; client 415 tests, game-api 53). Passe
  intégrée Campagne et Bataille sans panic (45 sections de vidage). Capture campagne : colormap,
  relief, neige, écus et accents OK ; frontières/remplissages absents, marqueurs du monde au-dessus
  du HUD, texte du briefing qui déborde. Capture bataille : unités livrées, bannières, cartes OK ;
  sol uni, ciel pâle.
- Vague 4 lancée (20 agents) : Opus drape, look2, towns, bsiege, integ ; Sonnet countryside,
  bdeploy, baudio, diplo2, progress, faction2, bdecor2, lifeplus, settings2, armies3, weather2,
  bunits2, vegetation2, codex2, bhud2. Stubs : campaign/{towns,countryside,progress}, battle/siege.
  Inventaire : 188 scripts Godot (47 k lignes) jamais cités dans client/ ni les notes bm-*.
- 15/10 ~4 h 50 : vague 4 entièrement fusionnée (20 lots ; client 599 tests, game-api 63).
  Passe intégrée : panic au démarrage de la campagne (commande `insert` sur une entité détruite,
  life/models.rs) corrigé par `try_insert` (84ea76b3e). Capture campagne : remplissages et
  frontières drapés visibles, HUD complet (boutons des panneaux), didacticiel ; modale rognée à
  gauche. Départ en vue parchemin (distance 2600 > seuil 1200), comme Godot.
  Inventaire : 126 scripts Godot (20 k lignes) encore jamais cités.
- Vague 5 lancée (20 agents) : Opus look3, qa (fumée + banc de perf), vqa (contrôle visuel, kit),
  bscene2 (rejeu, site historique), towns2 ; Sonnet armies4, court2, diplo3, hud3, province3,
  faction3, bdecor3, countryside2, vegetation3, water2, weather3, bhud3, audio3, progress2, menu3.
- 15/10 ~6 h 10 : vague 5 fusionnée sauf look3 (19 lots ; client 714 tests, game-api 70).
  Fusion de bscene2 cassée par menu3 (champ `site` de CampaignBattle) puis corrigée, comme la
  panique de descente de zoom (countryside/decor.rs, `try_insert`) et le bouton « Rejeux »
  (bd27d4dd3). Banc qa (release, machine chargée, d=30) : p50 16,6 ms, p95 32,9 ms, p99 46 ms
  contre Godot p50 19,8 ms, p99 41 ms (docs/wip/fl-fluidite-carte.md) ; à refaire au calme.
  Fumée : `client/tests/smoke.sh` (10 scénarios). Défauts visuels : docs/wip/bm-visual-qa.md.
- Vague 6 (10 agents) : fixqa (Codex vide, sol noir hors écran, F3/F4), fixui (chevauchements),
  towns3 (villes emblématiques 1:1), kit3 (champ de texte), countryside3, settings3, bdecor4,
  armies5, bspeech ; look3 toujours en cours.
- 15/10 ~8 h : le joueur confie toute la suite en autonomie (« fais tout en autonomie ») : enchaîner
  jusqu'à la parité, puis fusion dans main autorisée une fois la parité atteinte (fumée verte,
  contrôle à l'œil) ; toujours pas de push. Un agent de fusion permanent (Opus) fusionne les lots
  marqués `.bm-ready`. Vague 7 (9 lots, scripts Godot jamais repris) : Opus townfar ; Sonnet diplo4,
  econ, folk2, veg4, mapui, uiinfra, naval, settings4. En cours : campvis, bvis, perf (Opus),
  kbfree, bhud4, menu4 ; bspeech prêt à fusionner.
