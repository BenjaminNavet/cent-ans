# 0209 — Actions d'agents pilotées par une table `ActionSpec`

## Contexte
`sim-campaign/src/agents.rs` portait les 11 actions d'agents en `match` répétés (conditions, coût, chance, effets, libellés) et `agents.json` ne contenait que des constantes (`effects`). Ajouter ou régler une action imposait de toucher le code en plusieurs endroits.

## Décision
- `data/rules/agents.json` : table `actions`, une entrée `ActionSpec` par action (chance de base et par sceau, risque de mort, hostilité, coût `flat`/`per_man_percent`/`ransom`, cible `aim`, conditions nommées, effets de succès et d'échec, textes français).
- `data-model::entities::agent` : `ActionSpec`, `ActionCost`, `ActionAim`, `ActionCondition`, `ActionEffect` (enum à étiquette `kind`). Le moteur de `sim-campaign` lit la table et exécute les effets par un unique chemin.
- Le bloc `effects` disparaît ; seuls les paramètres de rançon restent au niveau racine (`ransom_price_*`), car ils dépendent du captif et non de l'action.
- Le champ `group` fusionne Truce et Parley en un seul bouton de la barre d'actions (première action disponible), sans fusionner les mécaniques : leurs conditions et effets diffèrent (proposition de trêve vs opinion).
- Schéma `data/schemas/agent_rules.schema.json` mis à jour.

## Conséquences
- Onze actions conservées (Incite, Counter, Denounce non supprimées : pas d'arbitrage de design pris ici). Les retirer revient à effacer une entrée de la table et une variante d'`AgentActionKind`.
- Le schéma type `conditions`, `success`, `failure` comme tableaux ; la validation fine des variantes est faite par `deny_unknown_fields` côté Rust.
