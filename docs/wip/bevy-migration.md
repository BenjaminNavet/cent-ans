# Migration vers Bevy (branche `bevy-migration`, worktree `../gp-bevy`)

Mandat (14/10 au soir) : nuit en autonomie complète, largeur maximale avec de nombreux agents Sonnet
contrôlés par Opus, **ni fusion dans `main` ni push**. Verdict : ADR 0347 ; architecture : ADR 0357.

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
