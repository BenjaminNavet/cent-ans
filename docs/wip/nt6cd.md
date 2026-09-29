# NT6 c et d — petites suites (IA au repos, réaffectation des touches)

Branche `feat/nt6cd-leftovers`. État : TERMINÉ.

## c — IA au repos pour se reconstituer

- Données : `postures.rest` de `data/ai/grid.json` (`enabled`, `below_percent` 60, `until_percent` 85,
  `watch_radius_km` 60), schéma et `AiRest` (data-model).
- Code : `ai::stances::rest_plan` (`RestPlan::{None, Rest{entrench}, Leave}`), appelé dans
  `campaign::plan_armies` juste avant l'attaque dans la bulle. Repos = rester en place (dans une place amie)
  ou camp retranché en rase campagne (`Stance::Entrenched`, validé par le noyau), en terres propres ou
  alliées, sans armée ennemie à moins de 60 km, place non assiégée, et si la reconstitution rendrait
  des hommes (trésor non vide). Sortie du camp à 85 % (hystérésis) ; en place amie, pas de marqueur :
  seuil bas seul (60 %).
- Tests : `core/crates/ai/tests/nt6c_rest.rs` (repos en place, camp en rase campagne, menace, sortie).
- Garde-fous, `century_probe 464 1 2 3 4 5` (normale), part de guerre FR-EN (cible 55-75 %) :
  avant 69 / 66 / 71 / 66 / 69 % ; après 71 / 62 / 72 / 64 / 72 % : dans la bande, seuil inchangé.
  `ai_beats_a_passive_ai_at_equal_forces` (sim-battle) passe ; `cargo test --workspace` vert.

## d — réaffectation des touches

- `game/scripts/ui/key_bindings.gd` (`KeyBindings`), réglage `input/bindings`, onglet « Commandes » de
  `settings_menu.gd` : bouton « Changer » (capture de la prochaine touche, Échap annule), conflit =
  avertissement + échange, « Rétablir par défaut ». Rechargé au démarrage par `Settings._ready`.
- Test `game/tests/nt6cd_keys_test.gd` OK ; smoke OK.
- `codex_pin_tooltip` et `tooltip_explore` (contextuelles, T partagé avec Technologies) hors conflit.
- Seule la touche principale d'une action se réaffecte (les touches secondaires, ex. flèches, restent).

## Points ouverts

- Pas de mesure en partie pilote du nombre d'armées au repos (seulement le probe centenaire).
