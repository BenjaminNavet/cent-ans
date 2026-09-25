# 0052 — Panique des chevaux sous les traits, haie tenue par les tireurs

Date : 2026-09-25. Statut : accepté. Suite du point ouvert de l'ADR 0046 (R4).

## Contexte

Depuis R4 (fin du tir à l'aveugle à travers les crêtes), la position anglaise type perdait à rapport
de forces serré : 3 arcs longs + 2 hommes d'armes à pied contre 3 chevaliers + 1 homme d'armes +
1 arbalète (`R4_FRENCH=heavy`), 0/32 sur toutes les crêtes. Trois causes, relevées sur trace :

1. **Le moral ignore la nature des pertes.** Une perte coûte `LOSS_MORALE_FACTOR` = 60 points de
   moral pour 100 % de l'effectif. Des chevaliers à 80 de moral qui perdent 90 % de leurs hommes sous
   les flèches restent à 26, au-dessus du seuil de déroute (20) : le tir use la charge sans jamais la
   briser. Or à Crécy, Poitiers et Azincourt, ce sont les chevaux blessés, cabrés, désarçonnant leurs
   cavaliers et refluant dans les rangs suivants, qui ont rompu les charges.
2. **La haie couvrait l'ennemi des archers.** Le couvert B5 (`hedge_between`) protégeait toute cible
   à moins de 14 m derrière une haie, même quand le tireur était lui-même collé à cette haie : les
   chevaliers arrêtés devant la haie de la crête, à 10 m, étaient couverts par la haie des archers
   postés à 8 m derrière elle.
3. **La mesure était un seul tirage.** Sur une géométrie fixe, les 32 graines rejouaient presque la
   même bataille (0/32 ou 32/32) ; un réglage donnait 40 → gagne, 45 → perd, 50 → gagne.

## Décision

- **Panique des chevaux** (`data/rules/missile_morale.json`, schéma
  `missile_morale_rules.schema.json`, module `missile_morale.rs`) : une troupe montée touchée par des
  projectiles perd en plus `mounted_panic_per_loss` = **40** points de moral par effectif entier tué
  au trait (proportionnel aux pertes), sauf si elle est déjà prise en mêlée (la presse du combat
  domine alors). Les troupes à pied ne changent pas ; les pertes en mêlée non plus.
- **La haie est à celui qui la tient** : une haie ne couvre la cible que si celle-ci en est plus près
  que le tireur. Les archers tirent par-dessus leur haie (Poitiers) ; une troupe tapie contre une haie
  reste couverte contre des tireurs éloignés.
- **IA : pas d'archers sans flèches contre des cavaliers libres.** Des tireurs à court de flèches qui
  rejoignent la ligne ne marchent plus sur des cavaliers qui ne sont pas déjà pris en mêlée (à
  Azincourt, les archers tombent sur des hommes d'armes déjà engagés). Sans cette règle, la panique
  changeait la trajectoire d'une bataille du test `ai_beats_a_passive_ai_at_equal_forces` (graine 2) :
  l'IA active, victorieuse, envoyait ses archers sans flèches un par un contre le dernier régiment de
  chevaliers adverse et perdait tout en déroute en chaîne.
- **Mesure** : `R4_JITTER=1` décale la crête et ses haies de −16 à +16 m selon la graine
  (`r4_survey.rs`), pour échantillonner plusieurs géométries.

## Mesures

`survey_english_position_against_knights`, `R4_FRENCH=heavy`, `R4_JITTER=1`, 32 graines,
victoires anglaises :

| Terrain | avant | panique 30 | panique 35 | panique 40 (retenu) | panique 45 | panique 60 |
|---|---|---|---|---|---|---|
| crête + haie | 4 | 14 | 16 | **24** | 28 | 24 |
| crête nue | 8 | 25 | 25 | **25** | 25 | 29 |
| haie en creux + crête | 15 | 22 | 22 | **22** | 25 | 29 |
| rase campagne | 0 | 0 | 0 | **0** | 0 | 32 |

(Colonnes panique avec la haie corrigée et la règle d'IA ; « avant » = main. La haie seule ne
change rien sans la panique, 4/32 : elle ne compte qu'une fois que les flèches peuvent briser la
charge.) À 60, les archers gagnent aussi en rase campagne contre une armée qui coûte 40 % de plus
(5 700 contre 4 050) : trop fort.

Bataille mixte sans site (`b6.rs`, `no_site_sim`, IA des deux côtés : 2 chevaliers + 2 hommes
d'armes + 2 arbalètes français, 5 800, contre 1 homme d'armes + 2 arcs longs + 1 régiment de chevaliers
anglais, 3 750), victoires françaises sur les graines 0-63 : avant **58**, panique 30 → 41,
35 → 39, **40 → 38**, 45 → 32. Toute valeur coûte ici : les chevaliers anglais font écran à
50 m devant leurs archers, et la cavalerie française, partie devant son infanterie, s'engage contre
eux sous les flèches. 40 est le compromis : la position défensive gagne environ trois fois sur
quatre, et l'armée plus riche garde l'avantage (59 %) en rase campagne. Les empreintes de
`battles_without_a_site_are_unchanged` (graines 3 et 11) passent aux Anglais.

Non-régression R2b (`survey_active_against_passive`, IA active contre passive, armées miroir,
128 batailles) : 100 → **90** (plaine 29 → 26, bocage 22 → 20, collines 25 → 22, montagne 24 → 22).
Coût attendu : le camp passif garde ses archers en place, et ceux-ci repoussent désormais la
cavalerie qui les charge de front.

Suites possibles (IA, hors de ce lot) : la cavalerie d'un assaillant ne devrait pas s'engager à
portée d'archers pourvus de flèches avant l'arrivée de son infanterie, ou devrait les prendre de
flanc ; cela rendrait aux Français une partie des victoires perdues en rase campagne.

## Conséquences

- La cavalerie lourde qui charge de front des archers pourvus de flèches se fait briser plus tôt ;
  les archers montés, les arbalétriers et l'artillerie en profitent aussi (chevaux blessés par
  n'importe quel projectile).
- L'auto-résolution de campagne n'est pas touchée (modèle séparé) : l'écart entre bataille jouée et
  bataille résolue grandit un peu pour les armées d'archers contre la chevalerie.
- Mêmes graines, trajectoires différentes pour toute bataille où des cavaliers essuient des traits.
