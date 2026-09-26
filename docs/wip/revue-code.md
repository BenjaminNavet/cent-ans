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
État : constats 1 à 18 traités (17 corrigés, 18c écarté). `cargo test -p sim-campaign -p ai` (profil isolé `review`) : 523 réussis, 0 échec, 6 ignorés ; clippy `-D warnings` vert. Les 12 tests de régression échouent sur les sources d'avant correctifs (b06cfc38) et passent après.
Prochaine étape : intégration (ff dans main) par le coordinateur.

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
État : terminé (7 constats traités, aucun écarté).

| n° | statut | commit | note |
|----|--------|--------|------|
| 1 | corrigé | 2b7973b6 | hors contact, un régiment qui a décroché repasse `Marching` (resté `Melee` tant qu'il est au contact et `disengaging`). |
| 2 | corrigé | b514f5b1 | avec une cible ordonnée, arrêt à portée seulement si elle est visible (`visible`) ; sinon on s'approche. `pick_shooting_target` : cible ordonnée intirable + tir à volonté → balayage du plus proche tirable. |
| 3 | corrigé | 2ed1b26a | en siège, `Withdraw` suit `grid_route` (A* rues/brèches) ; pas `route()` : sa branche échelles renvoie la ligne droite pour un assaillant, or on ne grimpe pas en se retirant. Sur le chemin de ronde : inchangé (tout droit vers l'intérieur). Hors siège : inchangé. |
| 4 | corrigé | a7013d12 | `Battlefield::clamp_inside` + `ai::FIELD_MARGIN` (10 m, comme `View::move_to`) : dégagement sur pieux et retrait des saignés (`react`), repli des tireurs (~1799, non cité mais même défaut), second flanc de `coordinate_flanks`. Effet : ces ordres, auparavant refusés près du bord, s'exécutent. |
| 5 | corrigé | a2e13448 | `primary_opponent` ignore les contacts morts depuis le calcul des contacts (tirs, feu, huile) ; le coup va à un ennemi vivant au contact au lieu d'être perdu. |
| 6 | corrigé | a6a3785f | perf, comportement identique : libellés de déroute/ralliement formatés après la boucle pour les seuls régiments concernés ; `standard_rules` en `Arc` (clone = compteur) ; `faction_name` du camp cloné seulement à l'alarme ou au pillage. Pas de nouveau test (messages couverts par battle.rs, ep10_rout.rs). |
| 7 | corrigé (partiel) | 752df541 | `ObstacleCache` (sim/pathing.rs) : verdict libre/bloqué de chaque cellule gardé entre les A*, par camp, rempli paresseusement (pas de précalcul complet : un incendie qui change la signature coûterait sinon une grille entière d'un coup), invalidé par la signature des chemins et par `siege_mut`/`set_scenario`. Le mémo local de `search` disparaît. Non fait (comportement modifié) : réutiliser l'A* quand le but bouge peu, string pulling moins fréquent. Gain non chiffré proprement (machine chargée ; siege/f5/sg1 pas plus lents). |

Tests : `cargo test -p sim-battle` vert après chaque correctif (39 binaires, 7 tests de régression
dans `review_fixes.rs`), clippy `-D warnings` propre.

Équilibre (sondages ignorés, 30 graines ; entre parenthèses les graines 1-20 des tests asserts) :
| état | Crécy | Azincourt | Poitiers | ep9b attaquant |
|---|---|---|---|---|
| avant | 25/30 (18/20) | 28/30 (18/20) | 20/30 | 4/10 |
| n° 1-7 | 23/30 (16/20) | 28/30 (18/20) | 19/30 | 4/10 |
| n° 1-7 sans le n° 2 | 24/30 | 26/30 | 21/30 | — |
Écarts de ±2 graines sans cause unique (bruit des scénarios IA) ; tous les tests d'équilibre
passent, Azincourt reste sous sa limite haute (18/20, limite 19). Rien à recaler.

Rejeux (EP13) : format inchangé, `REPLAY_FORMAT` non incrémenté (seule la structure du fichier
compte) ; `the_sample_file_still_reads` ne rejoue pas l'échantillon, il reste vert. Un rejeu
enregistré avant ces correctifs divergera et affichera « règles changées », comme après EP11.

Points ouverts :
- Dossier target partagé : `gp-review` et le dépôt principal produisent les MÊMES noms d'artefacts
  (chemins relatifs à l'espace de travail) dans `game_project/core/target/debug` ; une session sur
  le checkout principal écrase la lib de `gp-review` et inversement (j'ai vu mes tests tourner sur
  la lib du dépôt principal). J'ai travaillé dans un profil isolé :
  `cargo test --config 'profile.review.inherits="dev"' --config 'profile.review.debug=0' --profile review`
  (`target/review`, ~1,7 Go, supprimé à la fin). Les résultats de sim-campaign sur `gp-review` en `debug` sont à
  revérifier ainsi.
- `disengaging` retombe à faux au premier pas hors contact ; en pivotant, le rectangle du
  régiment peut retoucher l'ennemi et `enter_melee` annule alors la destination (vu en test :
  le décrochage en arrière d'une mêlée à la limite du contact échoue). Défaut préexistant, non
  corrigé (règle de mêlée, équilibre).
- n° 2 : les pavois attendent encore, à portée, une cible cachée (branche `pavise`), sans
  s'approcher.

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
| 13 | corrigé | `plan_peace` : buts de guerre non tenus ajoutés avec la clé `false` (avant les provinces hors but) ; `score <= 2 * SURRENDER_WAR_SCORE` (ADR 0025 § 5, « score ≤ -50 »). Pas de test dédié. |
| 9 | corrigé | naissance : l'enfant prend la faction du père, sauf si la mère est souveraine ou héritière de sa faction (maison du père inchangée, lieu de naissance = celui de la mère). |
| 10 | corrigé | `dynasty::check_marriage` (validation pure) extraite de `propose_marriage` ; appelée dans `check_treaty` (plus un même époux dans deux mariages refusé), donc `apply_treaty` n'applique plus la paix avant d'échouer ; `evaluate(Proposal::Marriage)` n'appelle plus `state.clone()`. |
| 18a | corrigé | `check_treaty` : doublons détectés par `articles[..i].contains(a)` au lieu de `serde_json::to_string` (même commit que 10). |
| 15 | corrigé | `naval::own_ships_lost` : par classe, seuls les premiers navires du dispositif jusqu'à l'effectif de la flotte sont propres ; les cogues louées perdues ne sont plus retirées de la flotte. Pas de test dédié (dispositif naval lourd à construire ; nv1/nv2 verts). |
| 16 | corrigé | blocus : un `BTreeSet` des provinces déjà comptées, un port bordant deux mers ennemies ne paie qu'un péage. |
| 17 | corrigé | `research_points_per_turn` compte les bâtiments des places tenues (`controller`), comme les taxes et l'entretien. |
| 18b | corrigé | `resolve_economy` et `faction_economy` passent le revenu déjà calculé à `administration_upkeep_for` (un seul `faction_income_effective` au lieu de deux). |
| 18c | écarté | cache de `shortest_path` des routes commerciales : le chemin ne dépend que de `GameData`, mais un cache global (clé route) serait faux entre plusieurs `GameData` (tests qui en chargent/modifient) et un cache dans `CampaignState` casserait `PartialEq`/sérialisation ; ni simple ni sûr. À faire plutôt en précalculant les chemins au chargement de `data.trade` (data-model). |
| 5b | corrigé | article `CedeSettlement` : passation par `hand_over` et garnison vidée, comme `cede_province` (le preneur ne reçoit plus la garnison, les recrues ni le chantier du donneur). |
