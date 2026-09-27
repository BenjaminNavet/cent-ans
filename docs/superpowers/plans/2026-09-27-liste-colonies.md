# Plan : liste des colonies (touche B)

Spec : `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` (validée le 2026-09-27).
Suivi : `docs/wip/colonies-liste.md`.

Deux lots séquentiels, chacun sur sa branche et dans son worktree, avec une cible cargo privée (voir les mémoires sur le dossier `target` partagé). L2 dépend du pont livré par L1. Les deux lots sont mécaniques une fois la spec lue, donc ils peuvent tourner sur Sonnet (`cent-ans-dev`).

## L1 — Cœur et pont (`feat/holdings-core`)

1. **Squelette, puis commit `wip:`**
   - `core/crates/sim-campaign/src/holdings.rs` : les structures `HoldingsOverview`, `ProvinceRow`, `SettlementRow`, `ConstructionRow`, `BuildOptionRow`, `SiegeRow` et l'enum `DangerReason` (dérivent `Debug, Clone, PartialEq, Serialize`), plus `pub fn holdings_overview(state, data, faction) -> HoldingsOverview` qui renvoie `todo!()` pour l'instant.
   - `pub mod holdings;` dans `sim-campaign/src/lib.rs`.
   - `core/crates/sim-campaign/tests/hl1_holdings.rs` : les 7 tests de la spec § 1, marqués `#[ignore]`.
2. **Implémentation**, en réutilisant sans rien dupliquer :
   - le revenu : `state.settlement_tax(data, id, tax_rate, &tech)`, avec `tech = research::faction_province_tech_effects`, comme `settlement_detail` dans `godot-bridge/src/campaign_sim_settlements.rs` ;
   - les options : `state.buildable(data, id)`, en ne gardant que `available == true` ;
   - la promotion : `data.buildings[&b].upgrades_from` présent dans `live.buildings` ;
   - l'agitation : `population::weighted_unrest(&province.population)` ; le seuil et le nombre de saisons viennent de `data.population_rules` ;
   - le plafond de garnison : `data.settlement_rules.garrison_cap[kind]` ;
   - l'ordre des colonies : celui de `province_settlements` (la cité d'abord).
   - Si le calcul du revenu ou du plafond est déjà écrit en ligne dans le pont, le déplacer dans une fonction du cœur appelée par les deux, plutôt que de le recopier.
3. **Tests** : retirer les `#[ignore]` et faire passer les 7 tests. Construire les états avec `CampaignState::new_1337` et la France, sur le modèle de `tests/c5_settlement_panel.rs`, et modifier directement `state.settlements` ou `factions[..].treasury` pour mettre en place chaque cas.
4. **Pont** : `core/crates/godot-bridge/src/campaign_sim_holdings.rs` (`mod` dans `lib.rs`, bloc `#[godot_api(secondary)]` comme les autres `campaign_sim_*`), avec `get_holdings_overview(faction) -> VarDictionary`. Les clés et les valeurs sont celles de la spec ; `danger_reasons` devient un `PackedStringArray` de clés texte.
5. **Vérifications, puis commit `HL1: ...`** : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, `core/build.sh` et `smoke.gd`. Fusion dans `main` en fast-forward, depuis un worktree séparé.

## L2 — Panneau Godot (`feat/holdings-ui`)

1. **Squelette, puis commit `wip:`** :
   - `game/scripts/map/holdings_controller.gd` (`HoldingsController extends Node`), avec l'API publique du registre des unités : `setup(map)`, `available()`, `toggle()`, `is_open()`, `refresh()`, `row_count()`, `province_row_count()`, `set_filter(key)`, `set_sort(key)` et `focus_settlement(id)`.
   - `game/tests/holdings_test.gd`, désactivé (il se termine immédiatement avec un message).
2. **Branchements**, sur le modèle exact de `units_ctl` :
   - `campaign_map.gd` : `var holdings_ctl: HoldingsController`, créé à côté de `units_ctl` (ligne ~189), et rafraîchi dans `refresh_all()` (ligne ~401) seulement s'il est ouvert. `refresh_all` est déjà appelé après chaque `_submit` réussi et à chaque tour, ce qui couvre tous les rafraîchissements de la spec.
   - `project.godot` : action `map_toggle_holdings` sur la touche B (`physical_keycode` 66). Vérifier d'abord qu'aucune action n'utilise déjà 66.
   - `map_ui.gd` : bouton « Colonies » après `UnitsButton` (ligne ~218), avec `_add_action_button`.
   - `shortcut_sheet.gd` : `["map_toggle_holdings", "Colonies (revenus, chantiers, menaces)"]`.
   - Exclusion mutuelle : `toggle()` ferme `map.units_ctl` s'il est ouvert. Ajouter la réciproque dans `UnitRosterController.toggle()`, sur une seule ligne.
3. **Panneau**, d'après la spec § 2 :
   - reprendre la mise en page, `_top()` et `_layout()` du registre ; largeur de 440 px ;
   - clic sur une colonie : `map.settlements_ctl.open_settlement(id, true)` ;
   - « ⌖ » d'une province : même chemin que `SettlementController._on_province_requested` (`map.picker.select_index`), plus la caméra ;
   - les montants passent par `money.gd` ;
   - les infobulles de danger traduisent les clés : siège → « Assiégée », occupied → « Occupée », revolt_countdown → « Révolte dans N saisons », unrest → « Agitation au-dessus du seuil de révolte ».
4. **Tests** : activer `holdings_test.gd` (cas de la spec § 2), puis lancer `holdings_test`, `unit_roster_test` et `smoke.gd`.
5. **Capture** : `game/tests/holdings_shot.gd` écrit `docs/img/holdings.png`. La session principale fait une seule lecture pour juger la lisibilité ; les sous-agents ne font pas de capture.
6. **Documentation, puis commit `HL2: ...`** : section « Liste des colonies (B) » dans `docs/manuel.md`, mise à jour de `docs/wip/colonies-liste.md`. Fusion en fast-forward.

## Critères de fin

- La touche B et le bouton ouvrent la liste. Les filtres et les compteurs correspondent aux données du cœur. Un clic ouvre le panneau de la colonie.
- Tous les tests sont verts (cargo, `smoke`, `holdings_test`, `unit_roster_test`), avec clippy sans avertissement.
- Les worktrees sont supprimés après fusion. Aucune dépense cloud.
