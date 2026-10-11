# RG13a — Conseil du royaume, règles du cœur (ADR 0341 §1-2)

Branche `worktree-agent-af58917677a35fc4a`, pas de fusion dans main.

## État
- Données : `data/council/offices.json` + `data/schemas/council_offices.schema.json`
  (5 charges, effets par point de compétence, `advice_kinds` = table alerte/indice/journal -> charge),
  test `tools/tests/test_council_offices.py` (tout genre d'alerte/indice de `game/` est rangé).
- data-model : `entities/council.rs`, `GameData::council: Option<CouncilOffices>`.
- sim-campaign : `council.rs`, `FactionState::council`, ordres `AppointCouncillor` / `DismissCouncillor`
  (`appoint_councillor` / `dismiss_councillor`), `EventKind::CouncilVacancy` (`council_vacancy`).
- Effets : taxe/troubles/expérience des recrues via `research::faction_tech_effects` (même crochet que les
  actes royaux) ; moral via `skills::character_effects` ; chancelier = ligne « Chancelier » dans
  `negotiation/context.rs` ; chambellan = phase `resolve_council` (reliquat en centièmes).
- Pont : `get_council(faction_id)`, `get_council_candidates(faction_id, office_id)` ; ordres par `submit_order`.
- SimFacade : `get_council`, `get_council_candidates`, `appoint_councillor`, `dismiss_councillor`.

## API pour la salle 3D (RG13c) — noms et champs figés
- `get_council(faction_id)` -> `[{office_id, office_name, skill, holder_id, holder_name, holder_skill,
  effect_text, advice_domains}]` ; `holder_id == ""` = vacant (alors `effect_text` = effet par point).
- `get_council_candidates(faction_id, office_id)` -> `[{character_id, name, skill_value, current_office}]`.
- Ordres : `{"type": "appoint_councillor", "office", "character"}`, `{"type": "dismiss_councillor", "office"}`.
- Genre de journal ajouté : `council_vacancy` (à styler dans `SeasonReport.KIND_STYLES` côté UI si voulu).

## Choix d'application de la spec (aucun écart d'API)
- Nommer un personnage qui tient déjà une autre charge est refusé (« une charge par personnage ») ;
  l'UI doit révoquer d'abord (`current_office` des candidats l'indique).
- Remplacer un titulaire ou le révoquer : -1 loyauté (`dismissal_loyalty_penalty`, donnée).
- Le joueur garde ses vacances (pas de remplissage automatique) ; l'IA les comble à chaque saison.
- Moral du connétable : points de moral (échelle 0-100) ajoutés au moral des armées de ses généraux.
- Les anciennes sauvegardes (sans `council`) sont complétées par `council::ensure_councils`
  (pont : `load_from_string`) ; sinon la phase saisonnière s'en charge.

## Reste à faire
- Rien côté cœur. Vérifications finales : cargo fmt/clippy/test, `core/build.sh`, import Godot, smoke.gd.
