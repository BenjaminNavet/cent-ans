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
- [ ] Vague 2 : D (runner, rejeu, mesures), E (6 bots).
- [ ] Vague 3 : F (équilibre ; arrêt avant passe longue), G (CI, docs, ADR 0347).

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
