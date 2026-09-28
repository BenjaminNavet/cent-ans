# 0103 — Compagnies de mercenaires (lot TW2-T3)

Date : 2026-09-28. Statut : accepté. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T3.

## Contexte

Avant T3, les unités marquées `mercenary` (routiers, arbalétriers génois, écorcheurs) se levaient
en ville comme les autres, bornées par leur période (`available_from` / `available_until`, lot
UR1), avec délai de recrutement, créneaux de la place et réserve de recrutement (T2). Total War
offre au contraire une réserve régionale de mercenaires, engageables par l'armée elle-même, où
qu'elle soit dans la région, sans délai, plus chers à l'achat et à l'entretien.

## Décision

1. **Module neuf** `sim-campaign/src/mercenaries.rs` (pas de changement de `economy.rs`, lot RS) ;
   chiffres dans `data/rules/mercenaries.json` (schéma `mercenary_rules.schema.json`, test
   `tools/tests/test_mercenary_rules_schema.py`), `MercenaryRules::default()` lit le fichier
   embarqué. IA dans un module neuf `ai/src/mercenaries.rs`.
2. **Régions** : le champ `region` des provinces (28 régions, nommées en français dans
   `region_names`). Une **bande** (`bands`) = une unité `mercenary`, des régions, une période, un
   plafond et une recharge par saison, une expérience à l'engagement :

   | Bande | Unité | Régions | Période | Plafond / recharge |
   |---|---|---|---|---|
   | Grandes Compagnies | routiers | France du Nord, du Centre, de l'Est, de l'Ouest, Aquitaine, Languedoc, Provence | 1360-1395 (après Brétigny) | 3 / 0,5 |
   | Compagnie Blanche | routiers | Italie du Nord et centrale | 1361-1394 (Hawkwood) | 2 / 0,35 |
   | Compagnies de Castille | routiers | Nord de l'Espagne, Castille, Aragon | 1365-1369 (Du Guesclin) | 2 / 0,4 |
   | Arbalétriers génois | génois | Italie du Nord et du Sud, Provence, Languedoc | 1337-1453 | 3 / 0,45 |
   | Génois des galées du roi | génois | France du Nord | 1337-1347 (clos des galées, Crécy) | 2 / 0,3 |
   | Soudoyers brabançons et hennuyers | **brabançons** (nouvelle fiche) | Pays-Bas, pays rhénans, France du Nord | 1337-1453 | 3 / 0,4 |
   | Archers des Marches écossaises | **archers écossais** (nouvelle fiche) | Écosse | 1337-1453 | 2 / 0,3 |
   | Armée d'Écosse | archers écossais | France du Nord, du Centre, de l'Ouest | 1419-1453 (Baugé, Verneuil) | 3 / 0,4 |
   | Écorcheurs | écorcheurs | France du Nord, du Centre, de l'Est, Languedoc, pays rhénans | 1435-1445 | 3 / 0,5 |

   Réserves stockées comme celles de T2 : seules les réserves entamées sont gardées
   (`CampaignState.mercenaries.pools`, bande → région → millièmes), absente = pleine ; nouveaux
   champs en `serde(default)`, `STATE_VERSION` inchangé.
3. **Ordre `HireMercenary { army, unit }`** (alias `unit_type`) : l'armée engage dans la région où
   elle se trouve ; la compagnie la rejoint **sur-le-champ**, à plein effectif, avec l'expérience
   de sa bande, sans toucher aux points de mouvement. Prix : **150 %** du coût de l'unité (× prix
   de la monnaie, × pourcentage de difficulté de l'IA). Limites : **2 par armée et 3 par faction
   et par tour**. Refus : **terre ennemie** (province tenue par une faction en guerre avec
   l'armée : les capitaines traitent avec un payeur, pas au milieu de l'ennemi ; tranché pour
   éviter qu'une armée d'invasion se regarnisse au pied des murs), armée enfermée dans une place
   assiégée, réserve épuisée (« +1 dans K saisons »), trésor insuffisant.
4. **Solde ×1,75** (`upkeep_percent` 175) : l'économie facture l'entretien ordinaire de chaque
   unité ; `resolve_mercenaries` (juste après `resolve_economy`) prélève la **surprime** (75 % de
   l'entretien des unités `mercenary`, garnisons à leur quote-part, × difficulté et prix), garde
   `premium_last_turn` par faction (affiché dans le panneau).
5. **Impayés** : si le trésor est négatif après la surprime, chaque compagnie de la faction
   **déserte** (50 %) ou **pille** la province où elle se trouve (+10 troubles, +8 dévastation,
   une fois par province et par saison) ; événements « Solde impayée : … ».
6. **Recrutement en ville retiré** pour les unités `mercenary` (motif « compagnie de mercenaires :
   à engager depuis une armée ») : un seul chemin, cohérent avec TW, la surprime s'appliquant à
   toutes les compagnies. Les unités `mercenary` du départ 1337 restent en place et paient la
   surprime. Le test pytest impose qu'une bande offre toute unité `mercenary` et que chaque bande
   reste dans la période de son unité.
7. **IA** (`ai/src/mercenaries.rs`, appelée après l'économie et avant les marches) : engage si
   **riche** (trésor restant après les ordres du tour ≥ max(4 000 £, 150 % d'une saison de revenu
   brut)) et si une armée est **menacée** (puissance ennemie à son ancrage ou une arête plus loin
   ≥ 80 % de la sienne) ; meilleure puissance par livre d'abord, jusqu'à 120 % de la menace, en
   gardant 75 % d'une saison de revenu, dans les limites et les réserves (comptées pour ne pas émettre d'ordre
   refusé). Une IA endettée licencie d'abord les unités les plus chères (`disband_for_debt`,
   inchangé) : les compagnies partent les premières.
8. **Pont et UI** : `get_mercenaries(army)` → `{region, region_name, hires_left, blocked,
   premium_last_turn, options[]}` (lignes au format de `get_recruitable`) et
   `hire_mercenary(army, unit)` (l'ordre passe aussi par `submit_order`). Bouton « Mercenaires »
   du bandeau d'une armée du joueur ; panneau `MercenaryPanel` (réutilise
   `PanelWidgets.fill_recruitable` : prix, solde, réserve, motif).

## Effet sur l'IA (sonde `ai/examples/t3_mercenary_probe.rs`, IA partout)

20 tours, `off` = aucune bande (règles d'avant T3 pour l'IA, qui ne levait pas de mercenaires en
ville dans ces parties). Départ 1337, ou calendrier avancé à 1360 (Grandes Compagnies).
Hommes en armées de campagne (moyenne des 21 relevés) et trésor de la France au tour 20 :

| Graine, départ | Compagnies engagées | France off → on | Angleterre off → on | Trésor France off → on |
|---|---|---|---|---|
| 7, 1337 | 7 (FRA 4, ENG 3) | 1 806 → 1 898 | 1 548 → 1 544 | 38 846 → 37 441 |
| 11, 1337 | 6 (FRA 6) | 1 753 → 1 978 | 1 436 → 1 486 | 36 558 → 27 571 |
| 23, 1337 | 12 (FRA 7, ENG 1, Flandre 4) | 1 860 → 2 227 | 1 615 → 1 516 | 45 388 → 35 726 |
| 7, 1360 | 9 (FRA 7, ENG 2) | 1 782 → 1 963 | 1 444 → 1 354 | 28 371 → 9 492 |
| 11, 1360 | 8 (FRA 8) | 1 868 → 1 962 | 1 421 → 1 353 | 30 570 → 17 022 |
| 23, 1360 | 17 (FRA 6, Flandre 10, ENG 1) | 1 939 → 1 825 | 1 639 → 1 401 | 39 566 → 28 046 |

Six à dix-sept compagnies en cinq ans, surtout pour la France (la plus riche) ; surprime payée
par la France 9 000 à 17 500 livres sur la période ; **aucun impayé**, aucune banqueroute nouvelle.
Réglage retenu après balayage (graine 7) : le seuil « riche » borne l'IA, pas la menace (80 % ou
60 % donnent les mêmes engagements) ; à 100 % d'une saison de revenu, 12 compagnies et 3 impayés
dès 1337 : trop ; à 150 %, 7-9 compagnies sans impayé (valeurs du fichier). Les trésors de l'IA
tournent autour d'une à deux saisons de revenu : l'ancien seuil de 3 saisons n'était jamais atteint.

## Conséquences

- Nouvelles fiches `unit_brabancons`, `unit_scots_archers` (icônes dérivées d'illustrations
  existantes, sans illustration propre : à peindre plus tard).
- Le budget de l'interface (`faction_army_upkeep`) ne montre pas la surprime : elle est dans le
  panneau « Mercenaires » et le journal ; l'intégrer au budget demande de toucher `economy.rs`
  (après le lot RS).
- Pistes : compagnies allemandes en Italie (Werner d'Urslingen), gallowglass d'Irlande, bandes
  navarraises ; licenciement volontaire d'une compagnie au lieu de sa désertion.
