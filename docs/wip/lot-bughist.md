# Lot BUG-HIST : batailles historiques qui ne démarrent pas

## Cause racine
Pas le `continue` de `historical.rs` (inactif ici) : l'invitation « Jouer le didacticiel de bataille ? »
(`menu/battle_prologue_invite.rs`) retient le premier lancement depuis le menu (custom ou historique)
tant qu'on n'a pas répondu. Les flags viennent de `~/Library/Application Support/Cent Ans/battle_prologue.json`
(dossier du joueur, même en run agent). Dump du smoke : `asking: true, held: historical:crecy:defender`,
état resté `MainMenu`. Le smoke ne vérifiait pas l'état final.

## Correctifs
- `smoke.sh` : dossier de prefs jetable (`CENT_ANS_PREFS`, `never_ask`), table `scenario_expect` (état attendu
  de chaque dump), dumps périmés supprimés avant chaque scénario.
- `historical.rs` : chaque échec silencieux -> `error!` + toast. `scene/mod.rs` : repli sur l'approximation
  personnalisée en `error!`.
- Test `historical_launch_tests` (construit chaque bataille x 3 camps).

- `run_bg.sh` : par défaut, prefs / saves / replays / settings (donc codex) dans un dossier jetable
  (variables `CENT_ANS_*` déjà posées respectées ; `CENT_ANS_REAL_PREFS=1` pour les vrais fichiers).
  Avant : un run agent lisait ET écrivait `~/Library/Application Support/Cent Ans/` (settings, codex, saves).

## État
Clippy vert ; smoke complet 11/11 vert avec contrôle d'état ; test `historical_launch_tests` vert ;
`cargo test --features dev` complet : voir le rapport.

## Prochaine étape (pause demandée)
Restait : `cd client && cargo test --features dev` complet (interrompu). Déjà verts : clippy, smoke 11/11, test historical_launch_tests. Ensuite : rapport final.
