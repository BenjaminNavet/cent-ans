# Critique QA — 22/10 (nuit)

Rôle : testeur QA. Aucun code modifié. Base : main `cfb117c4f`.

## Résultats des suites (exécutées cette nuit)
- `cd core && cargo test --workspace` : 1836 réussis, 0 échec, 17 ignorés (sortie complète).
- `cd client && cargo test --features dev` : 1389 + 12 + 3 + 2 + 1 + … réussis, 0 échec.
- `CENT_ANS_MUTE=1 client/tests/smoke.sh` : 10/10 (battle, campaign_battle, campaign_descent, campaign_turns, campaign, codex, crecy, faction_select, main_menu, siege), 3 à 51 s chacun.
- Constat général : base verte et large. Les risques sont dans les angles morts ci-dessous (sauvegarde, tests ignorés, erreurs silencieuses), pas dans des tests rouges. Lecture de code uniquement pour les constats, sauf mention « reproduit ».

## Constats

### 1. Écriture de sauvegarde non atomique (graves pertes de partie) — gravité haute, effort S
`core/crates/game-api/src/saves.rs:147` : `std::fs::write(&path, text)` tronque puis écrit. Un crash, une coupure ou un disque plein pendant l'écriture laisse un `.json` tronqué à la place de l'ancienne sauvegarde. Les autosaves tournent sur 5 emplacements (`flow.rs`, `auto_1..5`) et la sauvegarde rapide écrase toujours le même fichier : le slot détruit est précisément le dernier bon.
Étapes : lancer une partie, quicksave, tuer le processus (`kill -9`) pendant l'écriture (ou `ulimit -f` bas) ; relire : `read_save` renvoie « sauvegarde illisible ».
Correctif : écrire `<nom>.json.tmp` dans le même dossier, `sync_all`, puis `rename` (atomique sur APFS/NTFS). Idem pour `.meta.json` et le sidecar `.played`. Ajouter un test qui simule un tmp résiduel.

### 2. Échec d'autosave seulement journalisé, aucun retour joueur — haute, S
`client/src/campaign/hud/flow.rs` `autosave_after_turn` : `Err => warn!(...)`. Disque plein ou dossier en lecture seule : le joueur croit être sauvegardé pendant des dizaines de tours. `save_now` remonte pourtant l'erreur.
Correctif : `ToastRequest` d'avertissement (une fois par session, ou à chaque échec) + champ `last_autosave_error` dans le dump pour un test de smoke.

### 3. Pas de test de sauvegarde « ancienne version » réelle — haute, M
`STATE_VERSION = 9` et `OLDEST_MIGRATABLE_VERSION = 9` (`sim-campaign/src/save.rs`, `state.rs:50`) : la chaîne de migration est vide, les tests de `save.rs` migrent un JSON jouet (`add_field`, `rename_field`). Aucun fixture d'un vrai save v9 figé dans le dépôt. Dès le premier passage à v10, rien ne prouve que les saves des testeurs se chargent (champs `#[serde(default)]` oubliés = refus ou état faussé en silence).
Correctif : committer `core/crates/sim-campaign/tests/fixtures/save_v9.json` (partie 10 tours, graine 1337) + test « charge, joue un tour, re-sauve » ; règle : toute hausse de `STATE_VERSION` ajoute un fixture.

### 4. Noms de sauvegarde : collisions et noms vides — moyenne, S
`saves.rs:62` `file_stem` remplace `: / \ ? * " | % < >` par `_` : « a:b » et « a/b » écrasent le même fichier, sans avertissement, alors que la boîte de dialogue arme déjà une confirmation d'écrasement sur le nom brut. Nom vide ou d'espaces : `file_stem` renvoie `""`, d'où `.json` (fichier caché, `list_saves` le retrouve avec le nom vide). Noms > 255 octets, `.`/`..`, noms réservés Windows (`CON`, `NUL`, `AUX`), point final : échec d'écriture ou fichier inaccessible sur le port Windows (ADR 0087).
Correctif : `file_stem` renvoie `Result`/`Option` (refus du nom vide et des réservés, troncature à 100 caractères), la confirmation d'écrasement se fait sur le nom normalisé ; tests `proptest` ou tableau de cas.

### 5. `delete_save` laisse le sidecar `.played` orphelin — basse, S
`saves.rs:223` supprime `.json`, `.meta.json`, `.png` mais pas `<nom>.played` (`client/src/flow/history.rs:59`, `with_extension("played")`). Fuite de fichiers ; et pour un nom avec point (« partie 1.5 »), `with_extension` remplace « .5 » : le sidecar de « partie 1.5 » et celui de « partie 1.6 » entrent en collision (même `partie 1.played`) — l'historique des batailles d'une sauvegarde apparaît dans l'autre.
Correctif : construire le chemin par `format!("{stem}.played")` (pas `with_extension`) et le supprimer dans `delete_save` (le déplacer dans `game-api` pour que la règle soit unique).

### 6. Sauvegarde en deux fichiers non transactionnels (wrapper + sidecar) et miroir global — moyenne, M
`client/src/menu/saves.rs:511` `save_now` écrit le wrapper (qui contient déjà `client`) puis, séparément, `write_sidecar` depuis un `MIRROR` global au processus (`history.rs`). Si le second échoue ou si le jeu se ferme entre les deux, l'historique des batailles diverge de l'état. Le miroir est un état global partagé : les tests client qui touchent `load_into` / `save_now` en parallèle peuvent se perturber (flakiness latente, aujourd'hui masquée par l'ordre).
Correctif : mettre l'historique dans `wrapper.client` (un seul fichier, atomique via le constat 1) et supprimer le sidecar et le global ; migration : lire l'ancien `.played` en repli.

### 7. Tests de déterminisme et de relecture des seed bots ignorés par défaut — moyenne, M
17 tests `#[ignore]` : `seed-bots/src/runner.rs:459,478` (« plays 2 games x 5 turns twice », relecture d'un constat), `oracles_healthy.rs:11`, bots chaos/sailor/builder (`--ignored`), et `fk_map_scenes.rs:292` (165 s). Une régression de déterminisme (HashMap itéré, tri instable) ne sera vue que par la passe longue lancée à la main : `cargo test --workspace` reste vert. Or les sauvegardes, les replays et les constats `replay` reposent sur le déterminisme.
Correctif : un test rapide non ignoré (3 tours, 2 graines, 2 exécutions, même `save_json` digest) dans `sim-campaign/tests`, les lents en `cargo test --release -- --ignored` dans un job CI nocturne ; consigner la liste des ignorés dans `client/README.md`/`CLAUDE.md`.

### 8. Le smoke de client est `#[ignore]` dans `cargo test` et n'a pas de limite de temps globale — moyenne, S
`client/tests/smoke.rs:10` : ignoré (~4 min). Il faut penser à lancer `smoke.sh` à la main ; rien ne l'impose avant un commit (pas de hook). Les scénarios ont `--exit-after 6000` images, mais un blocage de `run_bg.sh` (`open -g -W` qui ne rend jamais la main) bloquerait le script. Observé cette nuit : 10/10, mais campaign_descent à 51 s sans marge de timeout documentée.
Correctif : `timeout 300` (ou boucle `kill` après N s) autour de chaque scénario dans `smoke.sh` avec raison « hang » ; hook `pre-push` optionnel qui lance `smoke.sh` si `client/` ou `core/crates/game-api` a changé.

### 9. Ordres perdus pendant la résolution du tour (compteur `orders_dropped`) — moyenne, M
`flow.rs` (`TurnFlow.orders_dropped`, commentaire : « those orders would be overwritten by the result ») : le worker travaille sur un clone de l'état ; tout écrit sur `Sim` pendant le tour est écrasé par le résultat. Le voile `OrderLock` + `orders_open` le prévient pour les entrées clavier, mais il faut que chaque système d'ordre pose `.run_if(orders_open)` à la main : un nouveau système oublié perd un ordre sans signal. Le compteur n'est qu'un témoin dans le dump, aucun test ne l'exige à 0 en fin de smoke.
Correctif : dans `smoke.sh`, échec si `flow.orders_dropped > 0` après `campaign_turns` ; mieux, centraliser les écritures dans une file d'ordres (`Sim::push_order`) qui refuse pendant `running()` et affiche un toast.

### 10. Charger pendant un tour en cours — moyenne, S (à vérifier)
`quick_keys` est gardé par `orders_open` (`saves.rs:118`), mais le dialogue de chargement (`saves.rs:495`) et « Continuer » du menu (`main_menu.rs:182`) appellent `load_into` sans cette condition vue à la lecture. Si le worker de fin de tour est actif (menu pause pendant le calcul d'un tour lent, 5 s mesurés sous charge, cf. `seed-bots-findings.md` « Tour lent »), le résultat du worker écrase l'état tout juste chargé : retour au tour d'avant le chargement avec `sim.player` d'une autre faction. Non reproduit (nécessite un script avec un tour long) ; hypothèse à confirmer par un scénario `end_turn` suivi immédiatement de `load`.
Correctif : refuser/mettre en file `load_into` tant que `flow.running()` (ou annuler le job et jeter son résultat via un compteur de génération `state_epoch`).

### 11. `unwrap`/`expect` en nombre dans le code non-test — basse à moyenne, L
`grep` brut : ~279 `unwrap()` hors tests dans `core/crates/*/src` (comptage approximatif, les modules `#[cfg(test)]` internes en comptent une partie) et plusieurs centaines côté `client/src` (ex. `world.try_query::<&SaveDialog>().unwrap()` dans le dump du menu). Un panic dans un système Bevy ferme le jeu sans sauvegarde.
Correctif : lint ciblé `#![warn(clippy::unwrap_used)]` sur `game-api` et `client/src/menu` d'abord (zone sauvegarde) ; hook de panique qui écrit une sauvegarde de secours `crash_<timestamp>` quand la campagne existe (S) — gain immédiat pour les testeurs.

### 12. Les tests ne couvrent pas la corruption de fichier — moyenne, S
`saves.rs` n'a que 3 tests (aller-retour, `client` absent, format de l'horodatage). Manquent : JSON tronqué, `state` vide, fichier de 0 octet, `state_version` futur (le code renvoie `VersionMismatch`, jamais testé de bout en bout), `list_saves` avec un `.meta.json` seul (sans wrapper) ou un meta qui ment, dossier inexistant. Le message d'erreur pour le joueur reste technique (« sauvegarde illisible : expected value at line 1 »).
Correctif : 6 tests tabulaires dans `saves.rs` + message en français distinguant « fichier abîmé » / « sauvegarde d'une version plus récente du jeu » / « trop ancienne » ; l'UI affiche la raison en toast.

### 13. `timestamp_now` : doc « local », implémentation UTC, tri lexicographique — basse, S
`saves.rs` : `SaveMeta.timestamp` documenté « Local ISO date », `timestamp_now()` en UTC sans fuseau. Deux saves écrites dans la même seconde (autosave puis « avant la bataille ») se départagent par nom, pas par ordre réel : « Continuer » peut charger l'autosave plutôt que `auto_battle`. Une horloge reculée (changement d'heure manuel) trie les saves à l'envers.
Correctif : ajouter une fraction de seconde (ou un compteur monotone) à l'horodatage ; corriger la doc ; test « deux écritures consécutives, la seconde est la première de `list_saves` ».

## Top 3
1. **Constat 1** — écriture de sauvegarde atomique (tmp + rename) : le seul défaut capable de détruire une partie ; correctif de 20 lignes.
2. **Constat 3** — fixture de sauvegarde v9 réelle et test de chargement : prévenir la rupture silencieuse à la prochaine montée de `STATE_VERSION`.
3. **Constat 2** (avec le hook de crash du constat 11) — ne plus échouer en silence : toast sur échec d'autosave et sauvegarde de secours au panic.
