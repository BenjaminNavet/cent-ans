# XP-AUDIENCE (R1) — budget d'attention et Audience de saison

Spec : `docs/audits/2026-10-29-experience.md` § C1, R1, « Lot XP-AUDIENCE ». Base : ADR 0810 (`season_story`), 0805/R1-UI, 0789.
ADR : `docs/decisions/XXXX-xp-audience.md` (à numéroter à la fusion).

## Plan
1. Données : `data/rules/agenda.json` (plafonds, tri, crises) + `data_model::AgendaRules` ; textes `data/ui/audience.fr.json` + schémas. FAIT.
2. Cœur : `game_api::agenda` (Affaires <= 3, À surveiller <= 5, Nouvelles résumées), ambitions en Affaire (`ambition_choice`),
   `ActAmbitionRow.locked`, `sim-campaign::chronicle` (une décision forte à la fois, report d'une saison), invariants
   `mission_offer_unseen` / `strong_decisions_open`. FAIT, tests verts.
3. Client : sonde `hud/attention.rs` (dump `attention`), `hud/audience.rs` (carte + Registre + dump `agenda`), offre de mission sur demande
   (`OpenMissionOffer`), décision plus ouverte d'office, carte « Que faire » absorbée (`hint::publish` retiré), cloche réduite à un
   compteur « Registre », sceau (infobulle) dit quelle Affaire bloque. Écrit ; clippy/smoke à valider.
4. Tests, fumée `campaign_attention`, seed bots rapide, ADR.

## Mesure (smoke `campaign_attention`, Angleterre, 10 saisons, Entrée x20)
Avant (sonde seule, comportement de main) : 61 ouvertures d'office en 11 vues de saison (33 toasts, 9 bandeaux Conseiller, 9 fenêtres de
décision, 10 fenêtres d'offre de mission, 0 modale). Dump : `out/before/before10.json`.
Après : voir plus bas.

## Points ouverts
- Conseils vocaux (`audio/advisor.rs`) et lettres scellées gardent leur canal.
- Registre : pas de filtre par rubrique.
