# sc/treaty — Composite des traités (CA1 + CA6)

État : `Proposal` supprimé ; `Treaty { articles }` (negotiation.rs) ; chaque `Article` sait `check`, `value`, `apply`, `label`
(negotiation/{check,value,apply,label,context,reasons}.rs). Poids numériques dans `data/ai/diplomacy.json` (`treaty_weights`).
Prochaine étape : adapter les tests (m5, jr_crusade, feudal_escalation, dp1…), ADR 0188, clippy/test.
