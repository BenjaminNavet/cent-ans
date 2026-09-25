# WIP — B5 Unités, navires, techniques (bulles partout)

Spec : `docs/design/2026-09-25-bulles-partout.md`. Branche : `worktree-agent-adf46adc6cef2e799`.

## Objectif
Une fiche Codex (`entity` = id) pour les 27 unités, 4 navires, 45 technologies ; `gameplay` chiffré
d'après `data/` et `core/crates` ; audit historique des JSON dans
`docs/histoire/audit-2026-09-25-unites.md`.

## État
- [x] Recherche des règles (capacités, effets de technologies, naval) dans `core/crates`
- [x] Fiches unités (27) : 20 nouvelles + 7 enrichies
- [x] Fiches navires (4)
- [x] Fiches technologies (45) : 29 nouvelles + 16 enrichies (tech_aqua_vitae passe de cdx_romarin à cdx_eau_de_vie)
- [x] Sources génériques des technologies remplacées, notes d'années corrigées
- [ ] Audit écrit + corrections JSON unités/navires + liens dans les descriptions
- [ ] Validateur Codex, pytest, cargo test

## Prochaine étape
Liens `[[cdx_…]]` dans les descriptions des unités/navires/techs, audit `docs/histoire/audit-2026-09-25-unites.md`, relecture.

Générateur : les textes « En jeu » sont produits par un script (chiffres lus dans `data/`), constantes de règles relevées dans `core/crates` (capacités `sim-battle/src/sim.rs`, économie `sim-campaign/src/economy.rs`, recherche `research.rs`, naval `sim-battle/src/naval/`). Champs sans effet en jeu (non mentionnés) : `shield_wall`, `wall_breach` (capacité), `mercenary`, `recruit_time_turns`, `cost.resources`, `tier`, `cost` des navires.
