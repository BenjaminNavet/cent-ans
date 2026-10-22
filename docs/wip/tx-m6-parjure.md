# tx-m6-parjure

Branche `tx/m6-parjure`, worktree `../gp-tx-m6-parjure`, ADR 0721.

## État
- Squelette : ADR, ce fichier, module `perjury.rs` (stubs), tests `tx_m6_*` ignorés.

## Prochaine étape
1. Structs `DiplomacyEvent` dans `data-model/src/entities/diplomacy_rules.rs` + schéma + `data/rules/diplomacy.json`.
2. `perjury::record_event` branché dans `war.rs` (remplace les constantes) ; `ally_attacked` ; exécution sous parole dans `ransom.rs`.
3. Aperçu dans `treaty_explain.rs` / `declaration_prestige_cost` ; IA `diplomacy_eval.rs`.
4. fmt, clippy, tests ; mesurer `wr_ai_diplo`.
