# WIP — C7 suite du général (« retinue » à la Medieval II), année de décès, mise en page de la fiche

Spéc : demande orchestrateur (lot C7 du plan `docs/design/2026-09-24-rapprochement-total-war.md`, suites de C3).
Branche : `worktree-agent-a29adc13935454195`.

## État : terminé, en attente de revue/fusion
- [x] Données `data/retinue.json` (15 compagnons, plafond 8) + schéma `data/schemas/retinue.schema.json`
  + `tools/tests/test_retinue_schema.py`.
- [x] `data-model` : `CompanionId` (`ret_`), `entities/retinue.rs`, `GameData::retinue` (fichier optionnel),
  contrôle des références (bâtiments, factions, traits, doublons).
- [x] `CharacterState::{death_year, retinue}` en `#[serde(default)]` ; version de sauvegarde inchangée (5).
  `death_year` rempli par `characters::kill` et à l'installation 1337 pour les morts d'avant 1337.
- [x] Règles `sim-campaign/src/retinue.rs` :
  - acquisition déterministe : hachage (graine, tour, personnage, compagnon, occasion), sans consommer
    le flux aléatoire principal ; au plus un compagnon par occasion, catalogue parcouru dans l'ordre ;
  - déclencheurs : victoire / bataille (`dynasty::on_battle_resolved`), siège gagné et chevauchée
    (`siege.rs`), rançon touchée par le souverain du geôlier (`chronicle::release_character`), saison
    de l'armée du général dans une colonie amie dont la province a le bâtiment (`turn.rs`, avant les morts) ;
  - effets : ajoutés à `skills::character_effects` sauf `Prestige`, payé chaque hiver au titulaire
    (souverain : via `dynasty::yearly_court_prestige`) ; `WoundRecovery` du général lu par
    `medicine::army_wound_recovery` (barbier-chirurgien) ; `ArmyUpkeep` du général lu par
    `economy::faction_upkeep` (banquier lombard, plancher −50 %) ;
  - mort : les compagnons héréditaires passent au fils aîné majeur de la même faction (sinon fille ; sinon
    au nouveau souverain si le défunt régnait), les autres partent ;
  - transfert : `Order::TransferCompanion { from, to, companion }` entre généraux de la même faction dont
    les armées sont dans la même colonie ; `transfer_targets` pour l'UI ;
  - ordre de débogage `DebugGrantCompanion` (smoke, captures).
- [x] Tests `core/crates/sim-campaign/tests/c7_retinue.rs` (11) : catalogue, déterminisme + plafond, flux
  aléatoire intact, conditions de faction et de bâtiment, effets, héritage, perte sans héritier, `death_year`
  des morts de 1337, transfert, sauvegarde sans les nouveaux champs, l'IA gagne des compagnons.
- [x] Pont `campaign_sim_retinue.rs` : `get_retinue_catalog`, `get_retinue_transfer_targets` ;
  `get_character` : `retinue`, `retinue_max`, `birth_year`, `death_year` ; `get_family_tree` : `death_year`.
- [x] UI : `retinue_row.gd` (`RetinueRow`, vignettes colorées par famille, infobulles riches, clic = confier
  à un général réuni) dans la fiche ; dates « 1310–1346 » (arbre et fiche, `FamilyTreeView.life_dates`) ;
  onglet « Suite » de l'encyclopédie (10 onglets) + paragraphe dans « Personnages ».
- [x] Mise en page : `CharacterSheet.fit_beside` (repli en une colonne < 1000 px de place, arbre de
  compétences dans un défilement horizontal), `CourtPanel.set_max_right` (l'arbre se rétrécit),
  placement dans `map_ui.layout_hud` ; sous-titres de la liste de la Cour tronqués (ellipse + infobulle).
  Avertissement « non-equal opposite anchors » supprimé (`set_anchors_and_offsets_preset`).
- [x] Smoke : 22 « smoke OK », 0 SCRIPT ERROR, 0 avertissement d'ancres.
- [x] Captures `docs/img/c7/fiche-suite-edouard-iii.png`, `docs/img/c7/arbre-dates-valois.png`
  (`godot --path game --script res://tests/c7_screenshot.gd`, ~10 min en debug).

## Notes / points ouverts
- Chances saisonnières divisées par deux après la capture (Philippe VI avait 6 compagnons en 14 ans) ;
  la seconde capture montre donc une suite plus fournie que la moyenne actuelle.
- Aucun défunt visible dans la capture de l'arbre vers 1351 (Philippe VI vit encore) ; les dates des
  défunts sont vérifiées par le smoke.
- L'IA ne transfère pas de compagnons (elle en gagne par les mêmes règles).
- Les échéances annuelles de rançon ne déclenchent pas `ransom_received` (seul le premier versement).
