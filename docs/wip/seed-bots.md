# Seed bots — note d'avancement

Spec : `docs/superpowers/specs/2026-10-13-seed-bots-design.md`. Plan :
`docs/superpowers/plans/2026-10-13-seed-bots-plan.md`. Worktree `../game_project-seed-bots`,
branche `seed-bots`. Feu vert du joueur reçu le 2026-10-13 : enchaîner toutes les vagues ; **arrêt
obligatoire avant la passe longue de référence** (lot F) ; aucun push.

## Écart avec la spec
- Pas de type `Action` : toutes les actions du joueur (diplomatie, recherche, édits, événements,
  offres…) sont déjà des variantes de `sim_campaign::Order` (~80, sérialisables). Les bots renvoient
  des `Order`. Les ordres `Debug*` sont exclus des candidats.

## État
- [x] Lot 0 — squelette (crate `core/crates/seed-bots`, `sim-campaign/src/invariants.rs`, profil
  `seed-bots`).
- [ ] Vague 1 : A (invariants + oracles 1-3), B (vue, candidats, random-legal, miroir), C (rapports, CLI, bandes).
- [x] Lot D — runner, rejeu, mesures (voir § Runner et § Mesures).
- [ ] Vague 2 : E (6 bots).
- [x] Lot F — métriques (`metrics.rs`), oracle `oracles/balance.rs`, tableau dans `report.md`/`report.json`, `seed-bots bands-propose <report.json>` (JSON proposé sur stdout). Passe longue de référence lancée le 13/10 (feu vert levé, ADR 0348 addendum).
- [ ] Vague 3 : G (CI, docs, ADR 0347).

## Prochaine étape
Vague 1.

## Ordres sans prérequis
Pas d'API de prérequis côté sim pour ces ordres : `legal_candidates` ne les génère pas (lot E, bot
`chaos`, ou futures requêtes) :
- Militaire : `Attack`, `Embark`, `SailRiver`, `Sortie`, `DemandSurrender`, `HireMercenary`.
- Armées : `CreateArmy`, `MergeArmies`, `SplitArmy`, `GarrisonUnits`, `DisbandUnit`, `RecruitInto`,
  `CancelRecruit`, `AssignGeneral`, `ChooseArmyTradition`.
- Bâtiments : `CancelBuild`, `CancelQueuedBuild`, `Demolish` (`demolition_blocker` existe, non branché),
  `SetSettlementVocation`.
- Cour : `AssignGovernor`, `DesignateHeir`, `ProposeMarriage`, `ProposeFactionMarriage`,
  `TransferCompanion`, `SendCharacter`, `LearnSkill`, `AppointCouncillor`, `DismissCouncillor`,
  `HireCaptain`, `RecruitAgent`/`MoveAgent`/`AgentAction`/`DismissAgent`, rançons et captifs, ordre de
  chevalerie, `ChooseMission`, `PreachPassage`, `DonateToChurch`, `ChooseObedience`.
- Féodal et diplomatie fine : `ProposeTreaty`, `DemandVassalage`, `ReleaseVassal`, `BreakAlliance`,
  `RevokeMilitaryAccess`, `SetEmbargo`, `BreakTradeAgreement`, `GrantTitle`, `DeclareCommise`, `Revolt`,
  `SwitchAllegiance`, `ArbitratePrivateWar`, `RequestPapalMediation`.
- Divers : `SetProvinceTax`, `SetCoinage`, `QueueResearch`/`DequeueResearch`.

Lot E (bots) : `sailor` construit `Embark` (requête `voyage::sea_voyages`, plus lointain port) et
`SailRiver` (`river_voyages`, plus coûteux) à la main, hors `legal_candidates` ; aucun candidat « naval »
n'existe dans les candidats légaux. Port = colonie avec une arête `sea` du graphe de mouvement. `Embark`
ne vérifie pas le mouvement plein (`EmbarkNeedsFullTurn` possible). `chaos` construit sans prérequis
`SplitArmy`, `MergeArmies`, `CancelRecruit`, `CancelBuild`, `DisbandUnit` (refus attendus). `turtle`
identifie les bâtiments défensifs par catégorie de données (fortification/militaire). `diplomat` n'a ni
mariage ni vassalité ni traités fins faute de requête.

Générés (famille) : Economy `Build`, `SetTaxRate`, `Research`, `SetEdict`, `SetDiet`, `RoyalAct` ;
Recruitment `Recruit` ; Military `MoveArmy` (cases atteignables), `SetStance`, `Assault` ;
Diplomacy `ProposePeace`, `ProposeAlliance` (filtrés par `check_treaty`), `DeclareWar` (relation
paix), `SendGift`, `AnswerOffer` ; Court `ChooseEventOption`, `ChooseEncounterOption`,
`ChooseCaptureOutcome`.

## Runner (lot D)
- Un fil par bot, un jeu par graine ; graine `k` joue `PLAYABLE[k % 3]` (France, Angleterre, Bourgogne).
  Générateur du bot : `derived_rng(seed, turn, index du bot dans bots::ALL, 0xB07)`. Bots inconnus de
  `make()` : ignorés, avertissement sur stderr.
- Vérification des refus : `legal_candidates` est **recalculé avant chaque ordre** (coût mesuré 5-6 % du
  temps d'un tour, donc gardé) ; un ordre est « vérifié » s'il figure (égalité) parmi les candidats
  courants ; seul un refus d'ordre vérifié est un constat. Exception dans `refused::check` : un
  `DiplomacyError::Refused` (la contrepartie dit non) est une issue de jeu, pas un bug.
- `play` unique pour la partie réelle et le rejeu (`Source::Bot` / `Source::Log`) : le rejeu resoumet
  `order_log` (refusés compris), refait la même vérification et les mêmes sauvegardes (tous les 10 tours
  et en fin), s'arrête après le tour du constat et ne garde que les constats de ce tour.
  Convention : `order_log[i]` = ordres du i-ème tour joué, celui du tour du constat inclus.
- Panique : la partie s'arrête (plan et soumission sous `guarded` sans limite de lenteur, `end_turn`
  sous `guarded` avec `slow_turn_ms`). Plafond `max_minutes` vérifié entre deux tours : `partial`.
- `RunResult.timings` (plan, candidats, ordres, fin de tour, invariants, sauvegarde) ; une ligne dans
  `report.md`, rien dans `report.json` (déterminisme).
- Tests : `two_runs_are_identical` et `injected_violation_is_replayed_at_its_turn` sont `#[ignore]`
  (quelques secondes, `cargo test -p seed-bots --release -- --ignored`) ; hook de test = fermeture
  appelée après `end_turn` qui corrompt l'état au tour 3.

## Mesures
Machine chargée par d'autres sessions (load average 16-23 sur ~10 coeurs), profil `seed-bots`.
- `run --quick` (8 bots, 1 graine, 30 tours) : 44 s ; cumul 289 s dont fin de tour 289 s sur 240 tours
  (~1,2 s/tour, machine chargée), candidats 17,5 s, plan 3,1 s, ordres 0,3 s, invariants 0,3 s,
  sauvegarde 0,3 s. Trop long : `--quick` passe à **20 tours** -> 31 s sous charge 16 (160 tours :
  fin de tour 205 s, candidats 13 s), ~20 s attendus machine calme.
- Longue réduite (8 bots, 1 graine, 150 tours) : **178 s** de mur (fils parallèles) ; cumul sur 1200 tours :
  fin de tour 1249 s (93 %), candidats 69 s (5 %), plan 14 s, invariants 1,7 s, sauvegarde 1,6 s, ordres 1,1 s.
  Soit ~1 s/tour en moyenne sous charge. Les 8 bots étaient disponibles, pas d'extrapolation.
- Passe longue par défaut : 10 graines x 150 tours ≈ 30 min : trop. Défaut retenu **4 graines**
  (couvre les 3 factions) x 150 tours ≈ 12 min sous charge, moins au calme ; le plafond 15 min reste.
  Pour les bandes d'équilibre (lot F), plus de graines pourront être demandées avec `--seeds`.

## Constats de la première vraie passe (non corrigés)
- Seule famille trouvée (quick et longue) : `SetEdict` / `SetDiet` déclarés possibles (`edict_options`,
  `diet_options`) mais refusés « déjà changé ce tour-ci » quand un 2e ordre vise la même province (ou
  une province du même groupe : ordre sur `prov_lancashire`, message « édit de Cheshire »). Le candidat
  reste listé après le premier changement : incohérence requête / règle (côté sim ou `candidates.rs`).
- Aucune panique, lenteur, invariant ni écart de sauvegarde sur 8 bots x 150 tours (graine 1, Angleterre).

## Métriques d'équilibre (lot F)
Sources (monde entier, pas seulement le joueur) : `revolts` = événements `Revolt` ; `bankruptcies` = événements
`Bankruptcy` (trésor vide, toutes factions) ; `war_turns_mean` = durée moyenne en tours des guerres entre factions
vivantes (paires `at_war_with`, guerre en cours à la fin comptée) ; `factions_eliminated` = factions vues vivantes
puis `alive == false` ; `leader_territory_share` = provinces possédées par la première faction (rebelles exclus) / provinces
de la carte. Seules les parties menées à terme comptent (pas de métrique si plafond atteint ou panique).
Mesure `--quick --balance` (20 tours, graine 1) : faillites 4-17 par partie (toute la carte, élevé), révoltes 0, guerres ~12-15 tours.
Procédure : passe longue -> `seed-bots bands-propose report.json > bandes.json` -> le joueur valide -> copie dans `data/balance/`.

## Clôture (2026-10-13)
Tous les lots (0, A-G) fusionnés dans `main` (9736828bb), worktree supprimé, non poussé. Passe rapide
de clôture : 31 s, 2 constats `SetEdict` (connus). Tests lents (déterminisme, rejeu) verts en
`--profile seed-bots -- --include-ignored`.

En attente du joueur :
1. Feu vert pour la passe longue de référence (4 graines × 150 tours, ~12 min), puis
   `seed-bots bands-propose <report.json>` → bandes proposées au joueur avant d'écrire
   `data/balance/seed_bots_bands.json`.
2. Tri des constats (dont `SetEdict`/`SetDiet` refusés « déjà changé ce tour-ci » sur un groupe de
   provinces) avant d'ouvrir `docs/wip/seed-bots-findings.md`.
