# DATA-GARDE (lot L-F, ADR 0829)

## État
- (1) FAIT : `data/schemas/manifest.json` + `tools/tests/test_schemas.py` ; 2 495/2 495 fichiers couverts (avant : 2 430 sur 2 494 ; le constat « 186 » était périmé), 0 invalide, 28 schémas ajoutés (dérivés des données, minimaux : types, clés requises, clés inconnues refusées).
- (2) FAIT : `GameData::load` lit ses fichiers indépendants sur 6 threads (`std::thread::scope`). Release, sonde `load_timing` (`cargo test --release -p data-model --test real_data load_timing -- --ignored --nocapture`) : avant 60/43/36 ms, après 62/21/22 ms (1er passage à froid, machine chargée). Le chargement était déjà bien sous 1 s : le gain est limité (~1,8x à chaud).
- (4) FAIT côté core : `data_model::texts::core_text` + `data/ui/core_messages.fr.json` ; migrés : tous les `#[error]` français de `orders/error.rs`, `ransom.rs`, `capture.rs`, et les `format!` français d'`invariants.rs`, `ransom.rs`, `capture.rs`. Test `sim-campaign/tests/core_texts.rs` : toute clé utilisée existe, aucune clé morte.

## Points ouverts
- Client (hors lot) : `menu/custom_battle.rs` army_cost, `army/basket.rs`, `settings/rows.rs`, `keymap.rs`, `court/view.rs`.
- Chaînes françaises restantes dans capture.rs/ransom.rs (littéraux simples, titres de journal) et `negotiation/value.rs`.
- Lint « aucune chaîne française nouvelle hors data » non écrit ; points (3) et (5) (règles client, constantes de rendu, `expect("checked")`) non traités.
- Schémas ajoutés à resserrer (bornes, énumérations).
