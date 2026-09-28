# 0099 — Règles de feu lues depuis `data/` et ordre « Incendier »

Date : 2026-09-28. Statut : accepté. Lot RS-F (`docs/wip/rs-f-battle.md`).
Aucune règle de combat ni de feu n'est changée : valeurs, tirages et ordre des tirages identiques.

## A. Règles de feu chargées au chargement des données

### Contexte
`sim-battle/src/fire.rs` embarquait `data/rules/siege_fire.json` par `include_str!` : changer un
réglage du feu imposait de recompiler (limite notée par S2, ADR 0008).

### Décision
- `FireRules::load(data_dir)` lit `rules/siege_fire.json` ; `FireRules::install(Some(r))` en fait les
  règles des batailles construites ensuite dans le processus ; `FireRules::current()` rend les règles
  installées, sinon la copie intégrée (`bundled()`, inchangée).
- La copie intégrée reste le **défaut des tests** et le repli : aucun test n'installe de règles, sauf
  `tests/rs_f_fire_rules_data.rs`, seul dans son binaire (l'installation vaut pour tout le processus).
- Le pont installe le fichier au **chargement disque des données** (`campaign_sim::load_shared_data`,
  donc `GameDataStore.load` et `CampaignSim.new_campaign`) et avant une bataille historique
  (`historical_battles::battle_data`). Un fichier illisible est signalé (`godot_error!`) et les
  règles intégrées restent en vigueur.
- `FireSystem::new` prend `FireRules::current()` ; `BattleSim::set_fire_rules` reste pour les tests.

### Conséquences
- Un réglage de `data/rules/siege_fire.json` vaut au prochain lancement sans recompiler.
- Un rejeu (EP13) ne stocke pas les règles de feu : il se rejoue avec celles du processus. Un rejeu
  enregistré avant une retouche du fichier peut diverger ; même limite que les autres règles
  intégrées.
- Seul le feu passe par ce mécanisme ; les autres règles de bataille (`*Rules::bundled()`) restent
  intégrées et pourront suivre le même chemin (`install` / `current`) au besoin.

## B. Ordre « Incendier » de l'interface

### Décision
- Le cœur décide : `BattleSim::burn_choice(side, units)` rend la maison ou la porte la plus proche,
  non encore en feu, à portée de torche d'un des régiments (tous ceux du camp si la liste est vide),
  avec le régiment porteur ; le test de portée est **celui de la commande `burn`** (fonction commune
  `torch_distance`), sans tirage. Refus : `NotASiege`, `Deploying`, `Finished`, `NoUnits`,
  `NothingLeftToBurn`, `NothingInReach` (deux variantes nouvelles de `CommandError`).
- Pont : `BattleSim.get_burn_order(side, units)` → `{siege, available, reason, command, target, unit,
  distance_m}` ; `command` est la commande `burn` à envoyer telle quelle.
- UI : bouton « Incendier » en bout de la barre des ordres du chef (`leader_orders_bar.gd`), aux
  sièges seulement, touche physique **I**, infobulle riche (cible et distance, ou raison du refus),
  grisé quand le cœur refuse. Aucune règle en GDScript.
