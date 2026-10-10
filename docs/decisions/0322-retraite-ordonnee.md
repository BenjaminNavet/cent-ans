# 0322 — Retraite ordonnée (lot TW retreat)

Statut : accepté.

## Contexte
En rase campagne l'IA de bataille ne se retirait jamais : seul le siège émettait `Command::Withdraw`, et une armée brisée finissait en `Broken`. Une retraite générale ordonnée (joueur ou IA) finissait en `Rout` avec `routed = true` et −20 de moral. Côté campagne, seul l'attaquant pouvait refuser la bataille.

## Décision
- IA (`sim-battle/src/ai/retreat.rs`, avant `plan_field`) : si, après `retreat_min_time` s, la part combattante est sous `retreat_share` et le rapport de force sous `retreat_ratio` (`data/rules/battle_ai.json`) si le général vit et si elle perd plus d'hommes que l'ennemi (seuil du défenseur `retreat_share_defender` plus bas : l'Anglais surnombré d'Azincourt tient son terrain, test historique `ep7_historical`) elle émet `Withdraw` pour les régiments, le général partant en dernier (quand il ne reste plus d'autre régiment valide). L'ordre est persistant dès qu'un régiment se retire.
- `BattleEnd::Withdrawal` (`check_end`) : le perdant n'a plus de régiment valide, aucun n'est en déroute, au moins un s'est retiré, le vainqueur tient. `routed = false`, `withdrew = true`, moral campagne `withdrawal_morale` (`battle_decision.json` : −8 / +3). Journal de bataille et de campagne distincts (« Retraite ordonnée »).
- Campagne : le défenseur du joueur peut se replier avant une bataille de campagne, hors siège et hors embuscade (`can_withdraw`) : moral `defender_withdraw_morale_loss` et traînards `defender_withdraw_straggler_percent` (`data/settlements/rules.json` § `retreat`), puis `retreat_beaten_army` (pertes de bataille nulles). L'attaquant garde le terrain.

## Conséquences
- Une armée qui s'arrête à temps garde ses effectifs et ne se débande pas, mais l'IA qui a perdu son général au combat ne bat plus en retraite (elle se brise comme avant).
- Pas de poursuite du fuyard ordonné au-delà de ce que la simulation 3D fait déjà ; la poursuite (lot ia-sieges #1) reste à faire.
