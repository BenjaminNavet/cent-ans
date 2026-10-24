# CREDIT-UI — interface du crédit et des impôts (ADR 0780)

## État
- [x] Données : `data/ui/finance_panel.json` + schéma + test_schemas.
- [x] `faction/finance.rs` : onglet Finances, ordres, alerte conseiller, banqueroute, raccourci Maj+J, dump `finance_panel`.
- [ ] cargo check / tests ciblés / clippy final
- [ ] Dump et vérif layout (1280x800, 1920x1080, échelle 1,25) ; captures (<= 5)
- [ ] Nettoyage du `target` du worktree

## Prochaine étape
Compiler, corriger, lancer le client hors écran avec `CENT_ANS_OPEN_FACTION` et lire `finance_panel`.
