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
| 3 | corrigé | `siege_leader` : le siège reste à l'assiégeant présent, sinon passe à un allié avec brèche et vivres ; `begin_siege` ne remplace plus le siège d'un allié. |
| 4 | corrigé | repli `Fallback` : une armée vidée par `decimate` est dispersée (`disperse_army`), comme en débandade. |
| 5 | corrigé | `SettlementState::hand_over` (fin du siège, file de recrutement et chantier perdus, sans remboursement) utilisé par `capture`, `cede_province`, la révolte (la garnison mutinée reste aux rebelles, inchangé) et la restitution des places occupées à la paix (`diplomacy`). |
| 6 | corrigé | la sortie affronte toute la coalition assiégeante (`settlement_coalition`, forces sommées) ; le siège n'est levé que s'il ne reste aucun assiégeant. |

## Corrections sim-battle

Branche `fix/code-review`, scope `core/crates/sim-battle` seulement. Tests de régression :
`core/crates/sim-battle/tests/review_fixes.rs`.
Build : profil cargo isolé `review` (voir « Points ouverts ») dans le dossier target partagé.
Référence d'équilibre avant correctifs (sondages ignorés) : ep7 `survey_all` Crécy 25/30,
Azincourt 28/30, Poitiers 20/30 ; ep9b `survey` attaquant 4/10.
Prochaine étape : constat 2.

| n° | statut | commit | note |
|----|--------|--------|------|
| 1 | corrigé | (ce commit) | hors contact, un régiment qui a décroché repasse `Marching` (resté `Melee` tant qu'il est au contact et `disengaging`). |

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

## Corrections GDScript bataille

État : terminé (non exécuté : smoke test à lancer à l'intégration).

1. Corrigé (e85e0935) : `battle_siege.gd` met à jour un pan ou la porte aussi quand `intact` change (mémorisé dans `view["intact"]`), même sous le seuil de 0,01.
2. Corrigé (d976a741) : `battle_soldiers.gd` crée les cadavres des dernières figurines d'un régiment anéanti (absent sans `left_field` ni `reserve`).
3. Écarté : en Godot 4, un `PackedFloat32Array` lu d'un dictionnaire partage la même référence (`PackedArrayRef`), pas un tampon en copie sur écriture ; l'écriture se fait en place (déjà supposé par `battle_gore._write(data, ...)`). Vider `layer["data"]` avant d'écrire ne changerait rien.
4. Corrigé (bb251b68) : `get_units` et `get_siege` lus une fois par image dans `battle_scene` ; `music.update(delta, units)`, `_frame_siege` pour la vue, le HUD, la sortie et l'audio. `battle_staging._sync_fires` (1 Hz) lit encore `get_siege` séparément.
5. Corrigé (08555828) : les couches d'un régiment passent à l'effectif quand il perd plus d'un quart des figurines, ce qui supprime la copie de complément à chaque image dans le cas courant.
6. Corrigé (f1b0be8c) : les imposteurs sont conservés d'un saut arrière à l'autre (atlas et cuissons en cours) ; `_bake` abandonne proprement si le nœud sort de l'arbre pendant un `await`.

Signatures modifiées : `BattleMusicDirector.update(delta, p_units = null)`, `BattleScene._check_sortie(siege)`, `BattleScene._build_soldier_layers(kept_impostors = null)`.

## Corrections GDScript carte/UI

Non exécuté (ni Godot ni cargo) : smoke test à lancer par l'orchestrateur à l'intégration.

| n° | statut | commit | note |
|---|---|---|---|
| 1 | corrigé | `3198fc03` | `_on_load` refuse pendant `ai_replay.playing` (toast) ; `_on_end_turn` sort après `await ai_replay.play()` si `sim` a changé. |
| 2 | corrigé | `94304f4d` | `MapUi._end_turn_pending` : une seule fin de tour tant que les deux images d'attente ne sont pas passées. |
| 3 | corrigé | `9cdb6547` | `siege_started` / `province_captured` dans `AudioDirector.EVENT_SFX` et le journal ; mock : `province_captured`, `recruited`. Types du mock encore absents de `EventKind` : `movement`, `appointment`, `skill_learned` (laissés, traités par l'UI). Les tests n'utilisaient pas les anciens noms (les `"siege"` restants sont des alertes UI, pas des événements). |
| 4 | corrigé | `89780e8e` | `_music_tween` tué avant un nouveau fondu ; le lecteur entrant repart de -40 dB. |
| 5 | corrigé | `80fd6c59` | `_on_save` passe par `SaveSlots.save` (fiche écrite seulement si l'état l'est) ; `FlowController._on_save_requested` ne fait plus que la vignette, sur `map.last_save_ok`. Nuance : la fiche et la vignette étaient déjà écrites par `FlowController` ; le vrai défaut était la fiche réécrite sur un échec. |
| 6 | corrigé | `4d14fbb3` | `_drop_freed_sources()` (tête de `_process`, clic, `pin_top_bubble`, `_remove`) + garde dans `_index_of_source`. |
| 7 | corrigé | `ffd2540d` | `_on_reset` reconstruit le contenu sur `self` (différé, onglet courant conservé) : les ouvreurs gardent un nœud valide. |
| 8 | corrigé | `fce38ff1` | `NewsLetters.push_news_batch(items)` : une reconstruction, un son ; utilisé par `MapUi.add_events`. |
| 9 | corrigé (partiel) | `38e47cc8` | Minicarte diplomatique : contrôleur lu une fois par province, et pas du tout sans faction sélectionnée. Pas de fonction groupée du pont pour le contrôleur. `alerts.gd` écarté : déjà une lecture par province, qui sert aussi au siège. |
| 10 | corrigé | `01ee9bca` | `_update_label_heights` seulement quand le seuil `weights.x > 0.35` bascule (`_label_near`) ; les hauteurs ne dépendent pas des poids continus ni du palier site ; l'alpha reste géré par `declutter`. |
| 11 | corrigé (partiel) | `fe60b198` | Seules les tuiles de hameaux d'une province dont la dévastation a changé sont marquées. Les lectures `get_province_state` restent : aucune fonction groupée ne donne la dévastation, et elle change aussi hors fin de tour (décisions de chronique). |
| 12 | corrigé | `0be83d77` | Plaque gardée quand le marqueur est réutilisé (même signature), `_style_plate` réappliqué ; plaques orphelines libérées. |

Signatures ajoutées (aucune signature existante changée) : `NewsLetters.push_news_batch(items: Array) -> void`, `CodexBubbles._drop_freed_sources() -> void`, `SettingsMenu._build() -> void`, `SettingsMenu._rebuild() -> void`, `DiplomacyPanel._controller_of(province_id: String) -> String` ; nouvelles variables `CampaignMap.last_save_ok`, `MapUi._end_turn_pending`, `AudioDirector._music_tween`, `SettlementLayer._label_near`.
| 7 | corrigé | défaite si le joueur ne contrôle plus aucune colonie (critère « terres » de `resolve_faction_deaths`) ; une armée sans terre ne suffit pas (conservateur : garde le texte « a perdu toutes ses terres » et le test m10). |
| 8 | corrigé | otage de traité : `ransom_terms = Hold` ; `own_prisoner` refuse un otage engagé (`Held`) donc ni parole, ni rançon, ni changement de conditions avant le terme ; `ai_ransom_orders` ne libère pas un captif `Hold` ; le héraut (`check_action` Ransom et `first_captive_held_by`) ne rachète qu'un captif aux conditions « argent ». Guerre : l'otage trahi redevient simple prisonnier (`ransom_terms = None`). |

## Corrections ai/pont

- 1 corrigé (0a073a45) : `GridPlanner::attack_order(army)` part de la puissance de l'armée elle-même ; les armées alliées à portée d'engagement ne sont plus comptées deux fois (avant : `power_at(ancre)` + boucle). Test `nearby_friendly_armies_are_counted_once` ; sonde 50 tours × 8 graines verte (un premier passage a vu une banqueroute anglaise graine 3 pendant que sim-campaign changeait en parallèle ; relance verte : marges minces).
- 2 corrigé (83fef09c) : `NavalBattleSim.tick` ignore les dt non finis ou ≤ 0 et plafonne à 600 pas par appel (comme `BattleSim`), arrêt si la bataille est finie.
- 3 corrigé (9aa18fda) : `GameDataStore.load` passe par le cache partagé de `CampaignSim` (`load_shared_data`) ; un seul chargement au lancement, avertissements conservés (journalisés une fois). API GDScript inchangée ; un second `load` du même dossier rend le cache (comme `CampaignSim`).
- 4 corrigé (97ba80f7) : `maps_in` rend un `Arc<[MapEntry]>` partagé ; `#[func]` inchangées.
- 5 écarté : `Ground::Pages` est un `HashMap<i64, Vec<u8>>` du crate `vegetation` (hors périmètre) ; et une clé de page n'est pas immuable si la pyramide change (autre carte), un cache par clé seule serait faux.
- Piège build : le dossier target partagé entre worktrees réutilise l'artefact `vegetation` d'une autre branche (sz1 : `relief_squash`) ; `touch core/crates/vegetation/src/*.rs` force la recompilation.
| 14 | corrigé | `ai_emissary` étape 1 : ne vise qu'un captif rachetable (conditions « argent ») dont le prix + coût de l'action tient dans le trésor ; sinon étapes 2 et 3. Pas de test dédié (IA d'agent, couverte par c6_agents). |
| 11 | corrigé | malus « otages abandonnés » déplacé dans `declare_war` : seulement si le donneur déclare la guerre au détenteur. Un donneur entraîné par un appel aux armes n'est pas puni (conservateur). |
| 12 | corrigé | `resolve_negotiation` : le registre de tributs d'une faction morte est vidé, elle ne paie plus. |
