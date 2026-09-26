# Revue de code globale (2026-09-26)

État : vague 1 lancée — 6 relecteurs en lecture seule (pas de build).
Tranches :
1. sim-campaign A : diplomacy, negotiation, agents, chronicle, dynasty, naval
2. sim-campaign B : orders, movement, state, battle_auto, battle_request, reste
3. sim-battle
4. ai, data-model, vegetation, godot-bridge
5. GDScript battle/ + map/
6. GDScript ui/, sim/, audio/, naval/ + tools/ Python

Prochaine étape : trier les constats, corriger les confirmés sur la branche `fix/code-review`
(worktree ../gp-review), tests, ff dans main.

## Corrections sim-campaign

Branche `fix/code-review`. Tests de régression : `core/crates/sim-campaign/src/review_tests.rs`.
Prochaine étape : constats 2 à 18 dans l'ordre (voir liste de la tranche 1-2).

| n° | statut | note |
|----|--------|------|
| 1 | corrigé | `movement::assign_captor` extrait ; appelé après un assaut (assaillant pris → défenseur) et une sortie (assiégeant pris → garnison) ; le gouverneur à la tête d'une garnison pris (`general_captured` du côté garnison) passe par `chronicle::capture_character`. |
| 2 | corrigé | `order_create_army` refuse une place assiégée (`OrderError::SettlementBesieged`, comme `GarrisonUnits`). |
| 6 | corrigé | la sortie affronte toute la coalition assiégeante (`settlement_coalition`, forces sommées) ; le siège n'est levé que s'il ne reste aucun assiégeant. |

## Corrections tools

Branche `fix/code-review`, worktree `../gp-review`, scope `tools/` uniquement. Tests :
`uv run --project tools pytest tools/tests` (154 passed, aucun échec après les 4 correctifs).
Aucun appel réseau ni build lancé (tout testé avec `httpx.MockTransport` / monkeypatch).

| n° | statut | commit | note |
|----|--------|--------|------|
| 1 | corrigé | `fix(tools): record billed OpenRouter refusals to the budget ledger` | `openrouter.request_image` lit `usage.cost` avant `_extract_image` et lève `ImageExtractionError(message, cost)` si aucune image n'est trouvée ; `generate_image` et `portraits.generate` attrapent cette exception et écrivent quand même la dépense (`budget.add_entry` dans un `finally`, uniquement si `spent > 0`). Tests : `test_generate_image_records_billed_refusal`, `test_generate_records_billed_refusal`. |
| 2 | corrigé | `fix(tools): make voice_tts record its real spend to docs/budget.md` | `voice_tts.main` tient désormais un cumul réel de l'exécution : `forgotten` (coût des clips repris par `--recheck`/`forget()`, sinon perdu) + `run_cost` (rejets compris, déjà accumulé via `RejectedClip.cost`). Ce total est vérifié contre le plafond global (`budget.check`) et écrit dans `docs/budget.md` via `budget.add_entry` dans un `finally` couvrant toute la boucle de génération, même en cas d'erreur API. Nouvel argument `--budget-path` (défaut `DEFAULT_BUDGET_PATH`) pour les tests. Tests : `test_main_records_real_spend_including_rejected_clips`, `test_main_recheck_keeps_forgotten_cost_in_the_recorded_spend`. |
| 3 | corrigé | `fix(tools): preserve text after the last budget table on save` | `BudgetLedger._load` capture désormais le texte qui suit le dernier tableau (`self.trailing`) au lieu de le jeter silencieusement quand la boucle se termine hors tableau ; `render()` le réémet tel quel après le dernier tableau. Test : `test_trailing_text_after_last_table_is_preserved` (aller-retour load/save sur un fichier qui se termine par une section `## Notes de clôture` sans tableau, y compris après un `add_entry`). |
| 4 | corrigé | `fix(tools): encode era-music tracks atomically to avoid stuck retries` | `convert_music`/`convert_layer` passent par `_run_ffmpeg_atomic` : ffmpeg écrit dans `<dst>.part`, remplacé par `dst` uniquement après succès (`Path.replace`), `.part` nettoyé dans un `finally` — un encodage interrompu ne laisse plus jamais croire que le fichier est prêt. Le bloc chalemie de `main()` vérifie maintenant l'existence de la piste musicale *et* de la couche de bataille séparément et ne retente que celle(s) qui manque(nt), au lieu de sauter les deux dès que le MP3 existe. Tests : `test_run_ffmpeg_atomic_writes_dst_only_on_success`, `test_run_ffmpeg_atomic_leaves_no_dst_and_no_part_on_failure` (la retente shawm/couche n'est pas testée unitairement : logique inline dans `main()`, dépendante du réseau — vérifiée par lecture de code). |

Rien d'écarté dans ce lot : les 4 constats étaient confirmés à la lecture du code.
