# ADR 0749 — Puits d'argent : ponction progressive sur l'épargne dormante (lot ECO)

Date : 2026-10-23. Statut : accepté. Amende les ADR 0183 (§ administration, trésors) et 0257 (opulence progressive). Paliers resserrés par l'ADR 0777 (1, 2 et 4 saisons de revenu).

## Contexte

Le juge (`out/juge/iter-02.md` C2, `iter-02-design.md` § 4) mesure, France passive : trésor 28 000 -> 111 667 en 4 saisons,
net +16 à +20 k par saison, 124 805 après 6 saisons. La « part du trésor qui dort » (ADR 0183/0257 : 20 % au-delà de
6 saisons de revenu brut, +15 % au-delà de 12) ne se déclenche jamais : le revenu brut de la France (~31 k) place le
seuil à 186 k, soit déjà 9 saisons de solde net. Mesure `campaign_probe --passive` (40 tours, graines 1-2, IA pour
toutes les autres factions) : le trésor de la France culmine à 207-226 k au tour 10-20.

## Décision

1. **Épargne dormante à paliers** (`data/rules/economy.json`, `EconomyRules::opulence`) : 15 % de ce qui dépasse 2 saisons
   de revenu, +20 % au-delà de 4 saisons, +25 % au-delà de 8 saisons (nouveau champ `opulence_tiers`, liste de paliers
   `{seasons, percent}` ; schéma `economy_rules.schema.json`). La ponction croît plus vite que le magot : un trésor ne se
   stabilise qu'à environ 5-6 saisons de solde net pour la France. Une faction qui dépense (recrute, bâtit, recherche)
   n'est pas touchée ; seule l'épargne oisive l'est. Le seuil reste relatif au revenu brut : les petites factions
   gardent leur réserve (2 saisons), et la règle de réserve de l'IA (`ai_event_reserve_seasons` 1) est inchangée.
2. **Effets de bâtiments sur l'armée** (données seules, aucun mécanisme nouveau) : `supply` (ravitaillement) sur
   greniers et grange dîmière (+10 / +6) et route royale (+5) ; `recruit_cost` -5 % au champ de montre (-8 % à
   l'arsenal, qui l'améliore) ; `recruit_slots` +1 au château (et à son amélioration, bastion d'artillerie). Sept
   bâtiments de plus touchent l'armée ou la logistique (entretien à payer).
3. **Outil** : `campaign_probe --passive` laisse la France inerte et sort la série `france_treasury` (tour, trésor,
   revenu, net) ; c'est la mesure de référence de ce lot (ADR 0246 : mesure ajoutée à l'outil officiel).

## Mesures (`campaign_probe --turns 40 --seeds 1,2 --passive --full`)

| Mesure | Avant | Après |
|---|---|---|
| Trésor France, tour 4 | 111 k / 108 k | 104 k / 103 k |
| Pic du trésor France (tour 10-20) | 207-226 k (10-11 saisons du net initial) | 135-136 k (6,8 saisons) |
| Trésor France, tour 40 | 136 k / 152 k | 81 k / 83 k |
| Banqueroutes (toutes factions, 40 tours) | 1 / 2 | 4 / 5 (<= 0,3 par faction et décennie, soit < 53) |

Les valeurs « après » sont encore trop hautes aux premiers tours (le trésor passe de 28 à 104 k en 4 saisons) : la
ponction ne mord qu'au-delà de 2 saisons de revenu, et le net de départ (+20 k) reste très supérieur à celui des autres
factions. Voir points ouverts.

## Conséquences / points ouverts

- Le solde structurel de la France (+20 k) n'est pas touché : la France reste riche, mais ne thésaurise plus. Un puits
  structurel (soldes obligatoires, rançons, dot, cadeaux de vassaux) demande un mécanisme nouveau et un rééquilibrage
  FR-EN (ADR 0100/0257) : non fait ici.
- Les effets de bâtiments sur l'armée sont modestes ; la refonte « logistique » (juge, chantier 3) reste à faire.
- Seed bots `--quick` : voir `docs/wip/eco-puits-argent.md`.
