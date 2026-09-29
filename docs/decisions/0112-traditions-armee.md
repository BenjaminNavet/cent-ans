# 0112 — Traditions d'armée (lot TW2-T5)

Date : 2026-09-28. Statut : accepté. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T5.

## Contexte

Seuls les personnages gagnaient de l'expérience (`skills.rs`, compétences du chef) et chaque unité
son propre niveau 0-10. Une armée n'avait pas d'identité : son nom suivait le chef
(« l'ost de Philippe VI ») et changeait avec lui ; rien ne récompensait de garder une armée
aguerrie plutôt que d'en lever une neuve. La reconstitution T2 (ADR 0102) rendait des hommes sans
toucher à l'expérience (écart à Total War noté pour T5).

## Décision

1. **État** : `Army.traditions` (`sim-campaign/src/traditions.rs`, `serde(default)`, omis quand
   vide) : xp cumulée, traditions choisies, nom et maison de bannière gardés, dernier rang annoncé.
   `Unit.experience_residue` (millièmes de niveau, `serde(default)`). Pas de changement de
   `STATE_VERSION` : une ancienne sauvegarde charge des armées sans traditions.
2. **Expérience d'armée** (`data/rules/army_traditions.json`, schéma
   `army_traditions_rules.schema.json`) : 10 par bataille livrée, +10 si gagnée, × le multiplicateur
   de l'issue nuancée CV3 pour les batailles rangées (héroïque ×2…) ; rien contre un ennemi de moins
   de 25 % des effectifs (pas d'écrasement de bandes). Trois points d'accroche couvrent toutes les
   batailles, auto-résolues ou livrées en 3D : `movement::apply_battle_result` (les deux camps),
   `siege::apply_assault_result` (assaillants contre la garnison) et la sortie de garnison
   (assiégeants). Seuils cumulés des rangs : **20, 60, 120, 200** (4 rangs ≈ 1, 3, 6 et 10 victoires).
3. **Traditions** : cinq branches de trois paliers, un choix par rang, le palier N exige le
   palier N-1 de la même branche (4 choix pour 15 traditions : spécialiser ou panacher).

   | Branche | Palier 1 | Palier 2 | Palier 3 | Accroche |
   |---|---|---|---|---|
   | Marche | +8 % | +8 % | +10 % de mouvement | `army_movement_allowance`, sur les points (pas sur les 3-4 étapes : l'arrondi les avalait) |
   | Intendance | +20 % | +25 % | +30 % du taux | `replenish`, facteur « Traditions de l'armée » de l'infobulle (plafond 40 % inchangé) |
   | Tir | +2 | +3 | +4 de tir | unités de tir, `side_from_army` (auto) et `side_setup` (3D) |
   | Assaut | 10 % | 15 % | 15 % de siège plus court | s'ajoute au `SiegeSpeed` du chef (`supplies_drain`) |
   | Tenue | +3 | +4 | +5 de moral | chaque unité, `side_from_army` et `side_setup` |

   Les effets de bataille passent par les mêmes bonus plats que les technologies et les bâtiments
   de levée (`research::boosted`), unité par unité : ils valent même sans chef, et le pont de
   bataille (`sim-battle`) n'a pas changé.
4. **Nom et bannière** : à la première expérience d'une armée menée par un chef, son nom
   (`army_name`) et la maison du chef (armoiries `HouseArms`) sont figés ; ils ne suivent plus les
   changements de chef. Une armée sans chef reste « l'ost de France » jusqu'à ce qu'un chef la mène
   au feu. Deux armées peuvent porter le même nom si l'ancien chef en mène une autre : accepté.
5. **Perte** : dissolution (dernière unité licenciée) ou destruction = traditions perdues ; un
   détachement (`SplitArmy`) part sans traditions ; une fusion garde celles de l'armée d'accueil
   (celles de l'armée absorbée disparaissent, comme dans Total War).
6. **Dilution des renforts** (`traditions::add_recruits`) : les hommes rendus par la
   reconstitution T2 et les levées locales des garnisons (`economy::reinforce_garrison`, une ligne)
   ramènent l'expérience de l'unité à la moyenne pondérée par les effectifs (recrues à 0), en
   millièmes de niveau pour que les petits renforts successifs s'additionnent (4 × 90/95 = 3,789).
   Les blessés soignés (H4) ne diluent pas : ce sont les mêmes hommes.
7. **Ordre** `ChooseArmyTradition { army, tradition }` (refus : tradition inconnue, déjà choisie,
   palier précédent manquant, aucun rang à dépenser, armée d'une autre faction).
8. **IA** (`ai/src/traditions.rs`, appelé par `plan_turn`) : dépense chaque rang aussitôt ; score
   par branche = poids de base (tenue 25, marche et intendance 20, tir et assaut 15) + 80 × la part
   de tireurs de sa doctrine de recrutement (Angleterre : tir) + 60 × la part d'hommes manquants de
   l'armée (armée entamée : intendance) + 30 en siège (assaut) + 10 pour une branche entamée
   (approfondir). Poids dans `army_traditions.json` (`ai`).
9. **Pont et UI** : `get_army_traditions`, `get_armies_with_pending_traditions`,
   `choose_army_tradition`, `debug_grant_army_xp` (`campaign_sim_traditions.rs`) ;
   `game/scripts/map/traditions_controller.gd` : bouton « Traditions (n) » dans l'en-tête du
   bandeau d'armée, panneau latéral (rang, barre d'expérience, armoiries, traditions et effets, une
   ligne par branche avec la prochaine tradition), toast au premier passage de chaque rang, titre du
   bandeau = nom gardé. Le joueur reçoit aussi une ligne de journal « … atteint le rang N ».

## Conséquences

- Garder une armée aguerrie vaut mieux que la dissoudre ; la reconstitution a un prix en qualité.
- `economy.rs` reçoit une ligne (dilution des levées de garnison) : conflit possible, mais trivial,
  avec le lot RS B.
- Tests : `sim-campaign/tests/tw2_t5_traditions.rs` (13), `ai/tests/tw2_t5_traditions_ai.rs` (2),
  `game/tests/tw2_t5_traditions_test.gd`, `tools/tests/test_army_traditions_rules_schema.py`.
- Reste ouvert : pas de sonde d'équilibrage sur une partie complète (rythme réel des rangs de l'IA) ;
  les traditions ne s'affichent pas encore sur le marqueur 3D de l'armée.
