# 0808 — ECO-R2a : la semonce féodale donne des soldats

Statut : accepté (boucle juge 2, lot ECO-R2a). Numéro attribué à la fusion.

## Contexte

Audit économie-politique du 29/10/2026 (`docs/audits/2026-10-29-economie-politique.md`, diagnostic n° 2) :
`feudal::summon_host` ne faisait que déclarer la guerre du vassal qui répond (`start_war`). L'ost
n'apportait aucun homme : un royaume de France entouré de vassaux n'alignait que ses propres armées.

## Décision

1. **Contingent.** Un vassal direct qui répond à la semonce lève une armée sous son drapeau
   (`Army::host_service = Some(HostService { liege, enemy, seasons, paid })`), dans sa capitale. Son
   entretien de saison vaut au plus `host.contingent_percent_of_income` % de sa recette de saison
   (`faction_income`), réparti selon `host.unit_mix` (parts du budget), entre `host.min_units` et
   `host.max_units` régiments (un vassal pauvre envoie au moins `min_units` du type le moins cher).
2. **Service.** `host.free_seasons` saison(s) aux frais du vassal (il paie l'entretien comme pour toute armée).
   Ensuite, si la guerre dure et si le suzerain peut payer une saison, le contingent passe à la solde du
   suzerain pour au plus `host.paid_seasons` saisons : chaque saison le suzerain verse au vassal
   `host.wage_percent` % de l'entretien. Sinon, à l'échéance, ou dès que la guerre cesse, le contingent
   **rentre** (l'armée est dissoute).
3. **Ordre `SummonHost { enemy }`** (joueur via `game_api::feudal::summon_host`, IA via
   `ai::feudal::plan_summon_host`) : le suzerain en guerre avec `enemy` rappelle ses vassaux directs ; ceux qui
   n'ont pas de contingent en lèvent un (même ceux déjà en guerre contre `enemy`). Délai entre deux semonces :
   `host.resummon_delay_seasons` (la semonce automatique de la déclaration de guerre l'amorce aussi). Un refus
   ouvre toujours un cas de félonie.
4. **Garde-fous.** Un contingent ne peut être fondu, scindé ni versé à la garnison (`OrderError::HostContingent`),
   sinon il échapperait à son terme ; on peut en congédier les unités.
5. **Mesure.** La sonde séculaire et la métrique `men_under_arms_peak` comptent aussi les hommes des contingents
   rattachés à l'ost de la maison (`host_service.liege`), alors qu'ils servent sous le drapeau du vassal.

Toutes les valeurs sont en données (`data/rules/feudal.json`, bloc `host`, schéma
`feudal_rules.schema.json`).

## Conséquences

- Les contingents obéissent à l'IA de leur vassal (le joueur ne commande pas les armées de ses vassaux) :
  « rattachée à l'ost » vaut pour la durée, la solde et la mesure, pas pour le commandement.
- Le suzerain paie la solde sur son trésor ; R1 (subsides) pourra en reprendre le financement.
- Sauvegardes : `host_service` et `FeudalState::host_summoned` sont `#[serde(default)]`.
