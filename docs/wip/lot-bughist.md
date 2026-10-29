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

## État
Vérifs complètes (clippy, tests, smoke) en cours.
