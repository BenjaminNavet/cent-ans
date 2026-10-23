# eclairage-repro : éclairage de bataille reproductible

## État
Code écrit, compilé, clippy/tests. Aucune vérification GPU faite (règle de session).

## Audit des entrées d'éclairage
- Météo de bataille : tirée dans le core (`Weather::draw(season, rng)` depuis la graine, `sim/setup.rs:40`) : déterministe pour une graine. Forçable par `CENT_ANS_BATTLE_WEATHER`.
- Heure de départ : `default_hour` 12,0 (`data/rules/battle_time_of_day.json`), sauf campagne (tirage graine) ou historique ; avance avec le temps simulé (0,2 min/s), déterministe à pas fixe. Nouveau : `CENT_ANS_BATTLE_HOUR`.
- Soleil / ambiance / ciel (`battle/decor/atmosphere.rs`, `battle/scene/lighting.rs`) : fonctions pures des données + clé d'heure + météo ; aucune horloge réelle (test `lighting_sources_do_not_read_the_clock`).
- `SystemTime` ailleurs : `audio/mod.rs:361` (rotation musicale), `flow/loading.rs:185` (carte de chargement), `replay.rs:161` (nom de fichier), `dataassets/mod.rs:435` : sans effet sur la lumière.
- Exposition automatique (`AutoExposure`, ultra) : suspect principal. Vitesse via `globals.delta_time` (temps simulé, fixe avec `CENT_ANS_FIXED_DT_MS`) mais la cible dépend de l'image rendue, donc de l'ordre de fin des chargements asynchrones. Nouveau : `CENT_ANS_FIXED_EXPOSURE=1`.
- Cube du ciel / carte d'environnement : contenu fonction de (heure, météo) seulement ; l'instant de pose varie, pas l'état final.
- Campagne : `CampaignLight` et brume lissées sur `Time<Real>` (fixe avec le pas fixe) ; `fog_clock` (brume matinale) dépend du temps écoulé depuis le tour : fixer la capture à un nombre d'images constant.

## Prochaine étape
Vérification GPU par la session principale (voir rapport).
