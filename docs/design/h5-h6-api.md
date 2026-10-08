# H5 « Monnaie » et H6 « Rançons et ordres de chevalerie » — règles, données et API du pont

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` § 5.1-5.2. Suivi : `docs/archive/chantiers.md`.
Règles : `core/crates/sim-campaign/src/coinage.rs`, `ransom.rs`, `chivalry.rs`. Pont :
`core/crates/godot-bridge/src/campaign_sim_h5h6.rs` (nouveau) et `get_faction_economy`.

## 1. Données

| Fichier | Contenu |
|---|---|
| `data/schemas/chivalric_order.schema.json` | Schéma d'un ordre (`ord_…`, déclaré dans `common.schema.json`) |
| `data/chivalric_orders/ord_*.json` | Jarretière, Étoile, Toison d'or, compagnie générique (§ 4) |
| `data/schemas/event.schema.json` | Nouvel effet `found_chivalric_order` `{order, faction?}` |
| `data/events/evt_ordre_de_la_jarretiere.json`, `evt_ordre_de_l_etoile.json` | L'option « fonder » fonde l'ordre |

Les paramètres de la monnaie et des rançons sont des constantes Rust, comme les autres règles
économiques du dépôt (`economy.rs`) ; seules les données historiques (ordres) sont dans `data/`.
Liens codex utilisés : `cdx_ordre_de_la_jarretiere`, `cdx_ordre_de_l_etoile`, `cdx_toison_or`,
`cdx_chevalerie` (fiches prévues dans `_todo.md` / `_event_links.md`) ; la monnaie s'appuie sur
`cdx_livre_tournois`, `cdx_nicole_oresme` (écrites), `cdx_mutations_monetaires`, `cdx_franc_a_cheval`
et `cdx_rancon` (déjà prévues dans `_todo.md`) : aucun identifiant nouveau à ajouter.

## 2. H5 — Monnaie

Ordre `set_coinage {level}` : `strong | sound (défaut) | debased | heavily_debased`, un changement par
année civile. État de faction (`serde(default)`) : `coinage`, `price_level` (100 = prix de 1337, entre
100 et 400), `coinage_changed_year`, `seigniorage_last_turn`, `recoinage_last_turn`.

| Niveau | Seigneuriage (% revenu fiscal) | Prix / saison | Refonte (% revenu fiscal) | Mécontentement bourgeois (cible) | Prestige du souverain / saison |
|---|---|---|---|---|---|
| `strong` | 0 | −2 (jamais sous 100) | 6 | −6 | +1 |
| `sound` | 0 | 0 | 0 | 0 | 0 |
| `debased` | 15 | +3 | 0 | +4 | 0 |
| `heavily_debased` | 30 | +8 | 0 | +8 | −1 |

- Le seigneuriage s'ajoute au revenu (ligne de revenu) ; la refonte s'ajoute à l'administration.
- Recrutement, entretien (armées, garnisons, bâtiments), construction et fondation d'un ordre
  × `price_level / 100`. Le revenu fiscal nominal ne suit pas : c'est la ruine à long terme.
- Inflation (`price_level − 100` = e) : bourgeois et clergé (rentes fixes) — cible de mécontentement
  +e/6 (max 25), cible de richesse −e/8 (max 20). Les paysans et la noblesse ne sont pas touchés.
- Calibrage : royaume dont l'entretien ≈ 70 % du revenu. `debased` rapporte ≈ 2 ans puis coûte ;
  `heavily_debased` s'équilibre en moins d'un an ; effacer 30 points d'inflation demande
  15 saisons de monnaie forte.
- IA (`coinage::ai_choose_coinage`, deux planificateurs) : trésor < −4 saisons de revenu et prix < 200
  → `heavily_debased` ; dette et prix < 160 → `debased` (jamais plus fort que l'actuel) ; trésor ≥
  4 saisons et prix > 130 → `strong` (maintenu tant que prix > 105) ; sinon trésor ≥ 0 → `sound`.
- Journal (`coinage`) : changement de monnaie ; pour le joueur, chaque palier de 25 points de prix.

## 3. H6 — Rançons

Rang : souverain (base 10 000 l.), héritier (5 000), grand seigneur (titré ou prestige ≥ 30 : 1 500),
chevalier (400).

**Rançon = base × (1 + prestige/100) × richesse**, prestige borné à 0-200, richesse = revenu fiscal
de la faction du captif / 10 000 borné à 0,5-2 ; arrondie à 50 livres.

- `pay_ransom {character, installments}` (faction du captif) selon les termes du geôlier :
  - `money` (défaut) : 0 ou 1 = paiement intégral ; 2 à 6 = échéances annuelles, total +10 %, la
    première tout de suite ; le captif rentre à la première échéance (comme Jean II après
    Brétigny), le reste devient une dette (`ransom_debts`) ;
  - `province` : la province exigée passe au geôlier (propriétaire et contrôleur, garnison et
    chantier perdus), sans prétention (simplification) ;
  - `hold` : refus ; `parole` : libération immédiate.
- Échéance impayée (trésor insuffisant) : dette +10 %, prestige du souverain −5, opinion du créancier
  −15 (20 tours), échéance repoussée d'un an. Journal `ransom`.
- `set_ransom_terms {character, terms}` (geôlier) : `{"kind": "money"}`, `{"kind": "province",
  "province": "prov_…"}` (province possédée et tenue par la faction du captif, pas sa capitale,
  voisine d'une province du geôlier), `{"kind": "hold"}`, `{"kind": "parole"}`.
- `release_on_parole {character}` (geôlier) : +8 prestige pour son souverain, +20 d'opinion de la
  faction du libéré envers lui (20 tours).
- Souverain captif : régence (même mécanique que le souverain mineur : mécontentement de régence,
  message « est captif : une régence gouverne »), prestige −1 par saison.
- Cohérence avec les événements (`evt_rancon_david_ii`…) : toute libération passe par
  `chronicle::release_character` (efface aussi les termes) ; un personnage libéré n'est plus captif,
  donc un second ordre ou événement de libération est sans effet (pas de double paiement).
- IA : paie comptant si le trésor vaut deux rançons ; souverain ou héritier en 4 échéances si la
  première laisse une demi-saison de revenu ; cède une province seulement pour son souverain ;
  comme geôlier, libère sur parole les simples chevaliers d'une faction avec laquelle elle est en paix.

## 4. H6 — Ordres de chevalerie

Ordre `found_chivalric_order {order}` ; un seul par faction ; l'ordre historique d'une faction lui
est réservé, les factions sans ordre historique fondent l'ordre générique.

| Id | Nom | Faction | Dès | Coût (l.) | Prestige requis | Membres (jeu) | Effectif historique | Loyauté | Moral | Prestige fondation / an |
|---|---|---|---|---|---|---|---|---|---|---|
| `ord_garter` | Jarretière | Angleterre | 1344 | 2 000 | 10 | 6 | 26 : souverain, prince de Galles et 24 chevaliers | +10 | +8 | +10 / +2 |
| `ord_star` | Étoile | France | 1351 | 2 500 | 10 | 10 | ≈ 500 prévus | +8 | +5 | +8 / +2 |
| `ord_golden_fleece` | Toison d'or | Bourgogne | 1430 | 3 000 | 15 | 6 | 24 + le duc, puis 30 + le duc (1433) | +12 | +6 | +12 / +3 |
| `ord_court_company` | Compagnie chevaleresque de cour | autres | — | 1 500 | 10 | 4 | 12 à 50 selon les ordres | +8 | +4 | +6 / +1 |

- Membres nommés automatiquement : hommes adultes, libres, vivants, de la faction, hors souverain
  (chef de l'ordre), par mérite = prestige + 5 × commandement + 2 × batailles (égalité : ordre des
  ids). Complétés chaque saison ; la loyauté est gagnée une fois à la nomination.
- Moral : un général membre ajoute `member_morale` (effet `army_morale` de `character_effects`,
  donc en bataille automatique comme en 3D).
- Prestige annuel du souverain en hiver.
- Mauron : si la moitié des membres est perdue (morts ou captifs) en une saison, l'ordre est
  brisé (`collapsed`) : plus de bonus, souverain −15 prestige. Simplification : seuls les généraux
  combattent comme personnages, la règle compte donc les pertes de la saison, pas d'une bataille.
- Événements : l'effet `found_chivalric_order` fonde l'ordre sans coût supplémentaire (l'option porte
  déjà trésor et prestige) ; sans effet si la faction a déjà un ordre ou si l'ordre est pris.
  Validé par `event_check.rs` (ordre connu, faction connue).
- IA : fonde le premier ordre disponible si le trésor garde ensuite 4 saisons de revenu.

## 5. API du pont (`CampaignSim`)

Ordres via `submit_order` :

```gdscript
sim.submit_order({"type": "set_coinage", "level": "debased"})
sim.submit_order({"type": "pay_ransom", "character": "chr_jean_de_normandie", "installments": 4})
sim.submit_order({"type": "set_ransom_terms", "character": "chr_david_ii", "terms": {"kind": "province", "province": "prov_lothian"}})
sim.submit_order({"type": "release_on_parole", "character": "chr_david_ii"})
sim.submit_order({"type": "found_chivalric_order", "order": "ord_star"})
```

Erreurs (français) : `la monnaie a déjà été changée cette année (1337)`, `la monnaie est déjà à ce
niveau`, `ce personnage n'est pas captif`, `ce captif n'appartient pas à votre faction`, `ce captif
n'est pas détenu par votre faction`, `son geôlier refuse toute rançon`, `nombre d'échéances invalide
(1 à 6)`, `trésor insuffisant : …`, `province impossible : …`, `ordre de chevalerie inconnu : …`,
`votre faction a déjà fondé un ordre`, `cet ordre appartient à une autre faction`, `cet ordre ne
peut être fondé avant 1351`, `prestige insuffisant : …`.

| Méthode | Retour |
|---|---|
| `get_coinage(faction) -> Dictionary` | (`""` = joueur) `{level, label, price_level, changed_this_year, seigniorage, recoinage, seigniorage_last_turn, recoinage_last_turn, options[{level, label, seigniorage, recoinage, inflation, deflation, burgher_unrest, prestige, current}]}` ; `seigniorage`/`recoinage` = prévision de la saison |
| `get_price_levels() -> Dictionary` | `{faction_id: price_level}` |
| `get_ransoms() -> Dictionary` | `{ours[], held[], debts[]}` ; captif : `{character, name, faction, captor, rank, rank_label, prestige, ransom, terms{kind, province}, plans[{installments, total, installment}], cedable_provinces[]}` ; dette : `{character, name, creditor, remaining, installment, next_due_turn, missed}` |
| `get_chivalric_orders() -> Dictionary` | `{founded: {order, name, founded_turn, collapsed, members[{character, name}]} ou {}, options[{id, name, description, cost, prestige_required, members, historical_members, member_loyalty, member_morale, founder_prestige, yearly_prestige, min_year, available, reason, founder}]}` |
| `get_faction_economy(faction)` | + `coinage`, `price_level`, `seigniorage` (inclus dans `projected_income`), `recoinage` (inclus dans `administration_upkeep`), `seigniorage_last_turn`, `recoinage_last_turn` |
| `get_events()` / `end_turn()` | nouveaux `kind` : `coinage`, `ransom`, `chivalry` |

Sauvegardes : tous les nouveaux champs (`FactionState.coinage`, `price_level`,
`coinage_changed_year`, `seigniorage_last_turn`, `recoinage_last_turn`, `ransom_debts`,
`chivalric_order`, `CharacterState.ransom_terms`) sont `serde(default)` ; `STATE_VERSION` reste 4.

À faire par la vague UI : panneau Monnaie (trésor), panneau Captifs (les deux camps, termes,
échéances), panneau Ordre de chevalerie, genres `coinage`/`ransom`/`chivalry` dans
`season_report.gd`, libellés dans `rich_tooltip.gd`.

## G1 — libérer contre rançon, fiche personnage
- Ordre `release_captive { character, ransom? }` : le geôlier libère son prisonnier contre `ransom` livres
  (défaut : la rançon calculée ; 0 = parole), payées comptant par la faction du captif si la somme ne dépasse
  pas la rançon calculée (`RansomTooHigh`) et que son trésor la couvre (`PayerCannotPay`) ; +3 prestige au
  souverain du geôlier.
- `get_character` expose `captor`, `captor_name`, `ransom`, `ransom_terms` et `ransom_action` (`pay` pour un
  captif du joueur, `release` pour un prisonnier du joueur). La fiche personnage affiche « Captif de … —
  rançon N livres » et le bouton « Payer la rançon » (`pay_ransom`, comptant) ou « Libérer contre rançon ».
