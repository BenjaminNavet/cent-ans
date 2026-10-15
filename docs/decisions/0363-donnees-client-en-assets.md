# 0363 — Données visuelles du client en assets Bevy typés

Statut : accepté (15/10, lot `bm/dataassets`).

## Contexte
Le client lit `data/` de façon dispersée : `data-model` pour les règles, et environ 100 lectures `std::fs::read` + `serde_json` à la volée dans les plugins pour les données visuelles (`fx/*.json`, `art/*.json`). Aucune ne se recharge : retoucher une valeur d'éclairage ou de ville impose de relancer le client. Le relevé `docs/research/bevy-outils-productivite.md` recommande `bevy_common_assets` (adopt) et `bevy_asset_loader` (try).

## Décision
- `data/` devient la source d'assets `data://` (`DataSourcePlugin`, ajouté avant `DefaultPlugins`, racine absolue de `paths::data_dir`). `game/assets` reste la racine par défaut.
- Chaque fichier visuel migré est un type `Asset` + `Deserialize`, chargé par `JsonAssetPlugin` (`bevy_common_assets` 0.18). Types de même extension `json` : le chargeur est choisi par le type du handle.
- Un état `DataLoad { Loading, Ready, Failed }` (`bevy_asset_loader` 0.28) collecte la collection `ClientData`. Fichier absent ou mal formé : état `Failed`, journal `DATA_ERROR: missing file: <chemin>`, section de dump `dataassets.error`. Le chargement ne bloque pas les écrans (il se fait en parallèle) ; les systèmes réactifs ne tournent qu'en `Ready`.
- Rechargement à chaud : la fonctionnalité Cargo `dev` active `bevy/file_watcher` (le binaire joueur non). Les systèmes réagissent à `AssetEvent::Modified` et mettent à jour la ressource vivante (`CampaignLightingLive`, `AtmosphereState.rules` + soleil, `TownsWorld.look`).
- Les JSON ne changent pas : `data/schemas` et son validateur Python restent la seule validation de schéma. Le type Rust est tolérant (champs inconnus ignorés) ; les valeurs invalides pour le schéma sont refusées en amont, pas par le client.
- Les types peuvent rester un document (`serde_json::Value` transparent) quand les lecteurs existants indexent par clé : on migre la source de lecture sans réécrire les consommateurs ; le typage fort vient fichier par fichier.
- Les règles de jeu restent dans `core/` ; seuls les fichiers visuels et d'interface du client sont concernés.

## Conséquences
- Éditer `fx/campaign_lighting.json`, `fx/atmosphere.json` ou `art/town_look.json` pendant que le client tourne (profil `dev`) change le rendu en environ 0,4 s (temporisation du moniteur de 300 ms comprise).
- Deux dépendances (`bevy_common_assets` JSON seul, `bevy_asset_loader` sans fonctionnalité) ; `notify` entre dans le profil `dev`.
- Les tests et dumps sans `main.rs` (application construite à la main) n'ont pas la source `data://` : l'état passe à `Failed` sans panique ; ils continuent à lire `data/` en direct (aucun test existant n'a changé).
- Le plan de migration des autres fichiers est dans `docs/wip/bm-dataassets.md`.
