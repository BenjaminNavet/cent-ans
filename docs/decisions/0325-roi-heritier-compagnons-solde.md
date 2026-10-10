# 0325 — Désignation d'héritier, suite de 40 compagnons, compteurs de missions, désertion sur solde impayée

Statut : accepté (lot TW `m2a`, rapport `docs/wip/tw/campagne-m2.md` § 3, top 1, 2, 3, 9).

## Contexte
Le critique « campagne vs Medieval II » relevait quatre manques peu coûteux : le joueur ne peut pas
choisir son héritier ; la suite des généraux n'a que 15 compagnons ; les missions ne comptent ni les
actions d'agents, ni les mariages, ni les rançons ; une solde impayée n'a qu'un effet sur le moral.

## Décision
- **Héritier.** Ordre cœur `Order::DesignateHeir { heir }` (`dynasty::designate_heir`) : membre vivant,
  libre et majeur de la maison du souverain (ou son enfant), de la même faction, qui n'est pas le
  souverain. Coût `designate_heir_cost` livres ; dans un royaume dynastique, si la loi de succession
  en désignerait un autre, le souverain perd `designate_heir_illegitimate_prestige` (les deux dans
  `data/rules/dynasty.json`). Un royaume électif ne subit pas ce malus. À la mort, `characters::succeed`
  lisait déjà `Faction.heir` en priorité (et `feudal::contested_succession` arbitre sous un suzerain).
  Le bouton vit dans `family_tree_view.gd` (`make_designate_button`), placé par `court_panel.gd`.
- **Suite.** `data/retinue.json` passe à 40 compagnons (25 nouveaux : sergent d'armes, fourrier,
  armurier, maître des archers, aumônier, notaire, trésorier, fauconnier…), mêmes déclencheurs,
  bâtiments et `EffectKind` qu'avant ; aucune règle nouvelle.
- **Missions.** Trois compteurs `MissionCounter::{AgentActions, Marriages, Ransoms}`, alimentés par
  `agents/orders.rs` (action réussie), `dynasty::propose_marriage` et `event_actions::release_character`
  (rançon > 0, payeur ou receveur). Trois modèles dans `data/missions.json` (`agent_work`,
  `royal_match`, `ransom_trade`).
- **Désertion.** `Army.unpaid_seasons` (`serde(default)`) croît chaque saison où le trésor de la faction
  est négatif, retombe à 0 sinon. À partir de `economy.json` `desertion_after_unpaid_seasons` (2), chaque
  unité perd `desertion_percent` % (5) de ses hommes par saison, avec une ligne de journal ; une armée
  vidée est dissoute.

## Conséquences
Le seuil et les pourcentages sont de l'équilibrage, réglables dans les données. Une faction IA en faillite
durable perd désormais ses armées : mesuré avec `campaign_probe` (voir rapport du lot).
