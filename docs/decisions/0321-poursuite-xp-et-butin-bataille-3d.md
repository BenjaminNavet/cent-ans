# 0321 — Poursuite, XP de régiment et butin après une bataille 3D (TW pursuit)

Statut : accepté.

## Contexte
L'auto-résolution poursuit les vaincus (`battle_auto.rs`, `pursuit_base` + cavalerie) mais une bataille jouée en 3D s'arrêtait net : les déroutés ne subissaient rien, aucun prisonnier de troupe, aucune expérience propre aux régiments, et les étendards pris / bagages pillés (`standards_taken`, `baggage_lost`) n'étaient que racontés dans la chronique, sans or ni prestige.

## Décision
- **Calcul pur dans le cœur** (`sim-battle/src/sim/pursuit.rs`, appelé par `BattleSim::outcome`) : aucun tirage, même bataille = même résultat. Batailles rangées seulement (pas les sièges) ; rien si `BattleEnd::Refused` ou `Lull`. Pour chaque régiment perdant : en déroute, part rattrapée = `routed_base + per_cavalry_ratio × (cavalerie gagnante valide / fuyards)`, plafonnée à `max_share` ; en retrait en ordre, `withdrawn_share` ; cavalerie en fuite × `fleeing_cavalry_factor`. Parmi les rattrapés, `captive_share` sont pris vivants, sauf si le vainqueur a donné « pas de quartier » (tous tués, aucun captif).
- **Champs `SideResult`** (serde default) : `captured` (prisonniers de troupe), `pursuit_losses` (tués par la poursuite, par unité), `unit_xp_milli`. Les rattrapés (tués et pris) sont ajoutés à `losses`/`total_losses` : la campagne applique donc les pertes par le chemin existant, sans double compte. Ils ne dépassent jamais les soldats encore vivants du régiment.
- **XP de régiment** : millièmes de niveau (`Unit::experience_residue`), `survival_milli` si le régiment tient ou se retire en ordre, `per_ten_kills_milli` par dizaine de tués (mêlée seulement : le cœur ne compte pas les tués au tir), `victory_milli` pour le vainqueur, plafonné par `max_milli`. Appliqué par `battle_spoils::apply_unit_xp` avant les pertes. Le niveau gratuit des vainqueurs (`retreat.rs`, moral positif) reste : l'XP mérite s'y ajoute et profite aussi aux vaincus qui tiennent.
- **Butin** (`spoils`) : chaque étendard pris rapporte `standard_gold` (pris au trésor du vaincu, jamais au-delà) et `standard_prestige` au souverain (plus `general_standard_prestige` pour la bannière du chef) ; un camp pillé rapporte `baggage_gold`. `standard_lost_prestige` (0 par défaut) permet de pénaliser le perdant. Chronique : une ligne « Poursuite » et une ligne « Butin ».
- **Captifs de troupe** : aucun mécanisme de rançon de troupe n'existe ; simple compteur (perte d'effectif + affichage à l'écran de fin). Les captifs nobles et l'exécution restent le lot WR captives (ADR 0303).
- Données : `data/rules/battle_outcome.json` (`pursuit`, `unit_xp`, `spoils`), schéma `battle_outcome_rules`.

## Conséquences
- Une défaite jouée coûte davantage qu'avant (les fuyards sont rattrapés) : de l'ordre de 12 % des déroutés sans cavalerie, jusqu'à 60 % avec une forte cavalerie ; à équilibrer à l'usage via le JSON.
- Parité d'ordre de grandeur avec l'auto-résolution (5 % + 20 % du ratio cavalerie sur tout le perdant) : ici seule la part en fuite est visée, donc un taux plus élevé par régiment.
- Sauvegardes anciennes : champs `serde(default)`, règles `serde(default)`.
- Reste : rançon de troupe en or, tués au tir dans l'XP, pénalité de prestige pour étendard perdu (valeur 0).
