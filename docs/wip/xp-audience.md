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

## Mesure (smoke `campaign_attention`, Angleterre, 10 saisons, Entrée x20, sonde `hud/attention.rs`)
| Ouvertures d'office en 11 vues de saison | Avant | Après |
|---|---|---|
| Toasts d'information | 33 | 0 |
| Bandeau Conseiller (`BandSay`) | 9 | 1 |
| Fenêtre de décision | 9 | 0 |
| Fenêtre d'offre de mission | 10 | 0 |
| Carte d'Audience (une par saison) | 0 | 11 |
| **Total** | **61** | **12** |

Avant : `out/before/before10.json` ; après : `out/smoke/attention_10.json` (non versionnés). Les toasts retirés : propositions
diplomatiques (8), évènements féodaux (3), décisions échues (9), missions ; la fenêtre Diplomatie ne s'ouvre plus d'office (elle ne
s'ouvrait pas pendant la mesure car une décision attendait). `ui_kit.widgets.total` au repos (carte repliée, tour 3) : 77 contre 90
avant (-14 %), 84 carte ouverte : le critère de -20 % n'est pas atteint (le Récit, le fil et le suivi de missions restent).
Affaires poussées : 3 au plus à chaque vue (contrôle `scenario_maxlen` de la fumée) ; au tour 10 : décision bloquante, offre de mission,
choix des ambitions.

## État final
- [x] Cœur, données, client, sonde, fumée (`campaign_attention`, `campaign_audience`), tests cœur et client verts, clippy cœur et client propres.
- [x] Fumée complète (13 scénarios) verte ; seed bots rapide : 8 parties, 0 constat ; propriété 200 graines x 12 saisons (test `#[ignore]`) verte.

## Points ouverts
- Conseils vocaux (`audio/advisor.rs`) et lettres scellées gardent leur canal.
- Registre : pas de filtre par rubrique ; actions des lignes « À surveiller » minimales.
- Critère `ui_kit.widgets.total` -20 % non atteint (-14 %).
