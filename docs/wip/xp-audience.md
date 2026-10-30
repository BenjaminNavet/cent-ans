# XP-AUDIENCE (R1) — budget d'attention et Audience de saison

Spec : `docs/audits/2026-10-29-experience.md` § C1, R1, « Lot XP-AUDIENCE ». Base : ADR 0810 (`season_story`), 0805/R1-UI, 0789.

## Plan
1. Données : `data/rules/agenda.json` (plafonds, tri, crises) + `data_model::AgendaRules` ; textes `data/ui/audience.fr.json` + schémas.
2. Cœur : `game_api::agenda` (Affaires <= 3, À surveiller <= 5, Nouvelles résumées), ambitions en Affaire (tour 1, nouvel acte), `sim-campaign::chronicle` (une décision forte par saison), invariant offre de mission.
3. Client : compteur d'attention (`AttentionLog`, mesure avant/après), `hud/audience.rs`, retrait des ouvertures d'office (offre, décision, conseil, rapport, toasts), ambitions grisées une fois le choix figé, dump `agenda`.
4. Tests, fumée, seed bots rapide, ADR.

## État
- [ ] squelette

## Points ouverts
