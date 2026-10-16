# bm-dataassets — données du client en assets Bevy (spike adopté)

Branche `bm/dataassets`. ADR 0363. Code : `client/src/dataassets/mod.rs`. Éditions hors dossier : `Cargo.toml` (2 dépendances, `dev` + `bevy/file_watcher`), `lib.rs` (1 ligne), `main.rs` (source `data://` avant `DefaultPlugins` + plugin), `battle/decor/atmosphere.rs` (`apply_sun` en `pub(crate)`).

## Résultats (mesures du 15/10, build debug `dev`, `--offscreen`, muet)
| Mesure | Résultat |
|---|---|
| Lecture synchrone avant (3 fichiers, 10 ko) | ≈ 2 ms ; ne bloque pas non plus aujourd'hui |
| Après : état `Ready` | atteint ≈ 1,5 s après la première image (debug, machine chargée), en parallèle du menu : aucun écran n'attend. Comparaison de bout en bout avant/après non mesurée (build de référence perturbé par la charge ; l'écart réel est de l'ordre du bruit) |
| Rechargement à chaud (édition → appliqué, lu dans le dump) | `campaign_lighting` 394 ms, `town_look` ≈ 400 ms, `atmosphere` (soleil de bataille, 14 000 → 28 000 lux) 446 ms ; dont 300 ms de temporisation du moniteur |
| Fichier absent | état `Failed`, `DATA_ERROR: missing file: …/town_look.json` (+ « Path not found » de Bevy), pas de panique |
| Tests | `cargo test --features dev` 887 + intégration ; clippy `-D warnings` propres ; 3 tests ajoutés (typage des vrais fichiers, lecteurs inchangés, fichiers listés présents) |

Dump : section `dataassets` (état, compteurs de rechargement, valeurs vivantes, mtime des fichiers pour mesurer le délai).

## Verdict : adopter
Les deux outils tiennent. `bevy_asset_loader` est léger pour ce besoin (un état, une collection) ; sa valeur est l'état d'échec propre. Limites vues :
- Le message d'échec du chargeur est générique : on liste nous-mêmes les fichiers absents (`FILES`).
- Plusieurs types sur l'extension `json` : le chargeur se choisit par type, OK, mais `load_untyped` serait ambigu.
- `register_asset_source` doit précéder `AssetPlugin` (donc `main.rs`).
- Un rechargement ne reconstruit pas la scène : `TownsWorld.look` est mis à jour, mais les villes déjà posées gardent leurs teintes jusqu'à reconstruction ; la lumière de bataille est, elle, appliquée tout de suite. À traiter par consommateur.
- À la fusion dans `bevy-migration` (après weather4, campvis2, campvis3) : `campaign_lighting.json` et `atmosphere.json` ont trois lecteurs synchrones côté campagne (`terrain/lighting.rs` `LightData`, `weather/atmosphere.rs` `FogLook`, `terrain/shadows.rs` `ShadowTuning`). L'asset devient `CampaignLightingDoc` (document brut + vue typée `CampaignLighting`) et `refresh_campaign_readers` reconstruit ces trois ressources par leur même `from_json` à chaque `Modified` de l'un des deux fichiers : un seul analyseur par consommateur, une seule source vivante. Les lectures synchrones restent pour le démarrage, les tests et les dumps (test `campaign_readers_agree_with_the_sync_loaders`).

## Plan de migration (ordre)
1. Fichiers à rechargement visible immédiat, petits, déjà en `Value`/structure : `fx/battle_decor_look.json` (`DecorLook`, brouillard/ambiance/ciel), `fx/horizon.json`, `fx/map_atmosphere.json` (+ `aurora`, `cumulus` qui le relisent chacun), `fx/campaign_lighting.json` quand le port existe, `fx/water_detail.json`.
2. Réglages d'animation/effets de bataille : `battle_gore`, `battle_animation`, `battle_staging`, `unit_looks`, `siege_engines`, `siege_fire`, `campaign_army_walk` (consommateurs `battle/units`, `battle/siege`, `campaign/armies`) ; ici les systèmes lisent aujourd'hui une fois à l'entrée de l'état : ajouter un « recalcul à `Modified` » par plugin.
3. Données de ville et de nature : `art/town_look.json` (reste : reconstruire à `Modified`), `town_maquettes`, `settlement_tiers`, `tree_species`, `forest_stands`, `ground_*`.
4. À laisser en lecture directe : tout ce qui passe par `data-model` (règles, cartes, pyramide, PNG/bin), les gros catalogues `dn_*` / `tx_*` (manifestes d'import, peu utiles à chaud), les YAML.
Pour chaque fichier : type `Asset` (document `Value` d'abord), ajout à `ClientData` et à `FILES`, système `Modified`, valeur dans la section de dump.

## Risques
- Dumps et tests headless : un `App` sans `DataSourcePlugin` met `DataLoad` en `Failed` (sans panique) ; les consommateurs ne doivent pas dépendre de `Ready` dans leurs tests, ils gardent le chargement synchrone en repli (`TownLook::load`, `read_rules`) tant qu'ils ne sont pas migrés.
- Un accès `Res<ClientData>` hors `Ready` panique : toujours `run_if(in_state(DataLoad::Ready))` ou `Option<Res<…>>`.
- `file_watcher` sur `data/` entier : surveillance récursive (pyramide de relief incluse) ; sans effet mesuré, à surveiller sur grosse arborescence. Jamais dans le binaire joueur.
- Chaque ajout de fichier alourdit la collection : un fichier absent met tout le jeu de données en `Failed` (voulu).

## Reste
- `CampaignLightingLive` (vue typée) n'a pas de consommateur : les lecteurs de campagne passent par `from_json` ; à retirer ou à faire lire par `LightData` si l'on veut une seule forme typée.
- Vérifier à l'œil qu'un rechargement de `campaign_lighting.json` en campagne s'applique (les ressources sont remplacées ; les systèmes les lisent à chaque image).
- Étendre aux lots 1 à 3 ci-dessus.

## Lot dataassets2 (UI)
Migrés vers `data://` (module `client/src/dataassets/ui.rs`, collection `UiData` ajoutée à l'état `DataLoad`) : `ui/illustrations.json` (-> ressource `ArtPlates`) et `ui/front_end.json` (-> `MenuData::front`, via `FrontEnd::from_value`). Adoption initiale en `OnEnter(DataLoad::Ready)` (les événements `LoadedWithDependencies` précèdent `Ready` : ne pas s'y fier), rechargement sur `Modified`. Lectures synchrones gardées pour le démarrage et les tests. Dump : section `dataassets_ui`.
Non migrés, à faire ensuite : `ui/battle_demos.json` et `rules/battle_time_of_day.json` (fonctions prenant `data_dir`, lues à la construction de l'écran), `fx/render_quality.json` (lu avant `Ready` par les réglages), `ui/encyclopedia.json` (codex, assemblé dans un gros `RulesBook`), `ui/tooltips`, audio (aucun JSON lu directement).
Hors dossier : `menu/front_end.rs` (`from_value`).
