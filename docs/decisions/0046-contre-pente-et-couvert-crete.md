# ADR 0046 — Contre-pente contre l'arc long, position « couvert × crête »

Date : 2026-09-25. Statut : accepté. Lot R4 (suivi : `docs/wip/r4-couvert-crete.md`), suite de R2b
(ADR 0020, `docs/wip/r2b-ia-relief.md`).

## Contexte

Après R2b, la ligne de vue ne gênait que les troupes sans la capacité `volley` : les archers à l'arc
long, les francs-archers, les archers montés et aussi les arbalétriers génois tiraient à travers
n'importe quelle crête, à pleine précision, sur une cible que personne ne voyait. La contre-pente de
R2b ne protégeait donc que des arbalètes « à vue », et l'IA l'abandonnait dès que l'ennemi tirait en
volée. Par ailleurs, la position défensive se choisissait en deux temps sans les comparer : le couvert
de B6 (haie, fossé, village) d'abord, sinon la hauteur de R2b ; une haie en creux l'emportait donc sur
une crête, et une haie sur une crête (la position anglaise type : Poitiers) n'était pas reconnue comme
meilleure que l'une ou l'autre. Enfin, l'attaquant avançait toujours, même quand il dominait un ennemi
fait surtout d'archers qui n'attendait que ça.

## Ce que disent les sources

- **Portée et angle de l'arc long.** Les répliques des arcs de la *Mary Rose* (tirages de 70 à 80 kg
  et plus) envoient une flèche de guerre lourde (~ 90-100 g) à environ 220-250 m, les flèches légères
  plus loin ; la portée maximale s'obtient autour de 45° (Strickland et Hardy, *The Great Warbow*,
  2005 ; Hildred éd., *Weapons of Warre*, Mary Rose Trust, 2011). Le jeu garde les portées des
  données (220 m pour l'arc long) et plafonne l'angle de départ d'une volée en cloche à 45°.
- **Tir tendu et tir en volée.** Aux distances courtes l'archer vise (trajectoire tendue) ; au-delà
  de ~ 150 m ou au-dessus d'un obstacle, la volée est une « pluie » tirée sur une zone : sa densité
  compte plus que la visée. Aucune source chiffrée n'établit la précision d'une volée tirée à
  l'aveugle derrière une crête : le facteur retenu (0,3 guidé par un allié, 0,2 de mémoire) est une
  estimation de jeu, choisie pour qu'une contre-pente divise nettement les pertes sans les annuler.
- **Crécy (1346).** Les Anglais tiennent le versant d'une croupe entre Crécy et Wadicourt, une aile
  appuyée à la Maye et au bois de Crécy, l'autre au village de Wadicourt ; les arbalétriers génois,
  sans leurs pavois restés aux bagages, tirent en montée et sont surclassés ; la chevalerie charge
  ensuite en montée à travers eux. → hauteur + glacis + flancs appuyés, archers qui voient venir.
- **Poitiers (1356).** L'armée du Prince Noir se poste derrière des haies et des vignes, sur une pente,
  un chemin creux pour seul passage ; les archers derrière la haie tirent sur les hommes d'armes qui
  montent à pied, et ceux de Salisbury, postés au bord d'un marais, prennent les chevaux de flanc.
  → la position anglaise type : haie sur une crête, flanc sur un marais.
- **Azincourt (1415).** Les bois d'Azincourt et de Tramecourt resserrent le front et appuient les
  deux ailes. **Verneuil (1424)** : en rase campagne, les pieux mal plantés en sol dur n'arrêtent pas
  la cavalerie lombarde qui tourne un flanc d'archers ; sans appui, le flanc est le point faible.
- **Contre-pente.** La doctrine est formalisée bien plus tard (Wellington, Buçaco 1810, Waterloo
  1815, contre l'artillerie et les tirailleurs) ; pour la guerre de Cent Ans, on n'a pas de récit
  d'une ligne qui se dérobe exprès au tir derrière une crête. C'est donc une extrapolation tactique :
  plausible (un tir en cloche sans observateur ne vise rien), mais pas une reconstitution.

## Décision

1. **Tir tendu, tir en cloche** (`core/crates/sim-battle/src/missile_arc.rs`, `sim/indirect.rs`,
   règles `data/rules/missile_arc.json`, schéma `missile_arc_rules.schema.json`) :
   - cible vue (ligne de vue du champ) → tir **direct**, précision pleine ;
   - cible masquée par le relief → seules les troupes `volley` dont le projectile figure dans
     `indirect_missiles` (les **flèches** : arc long, francs-archers, archers montés) peuvent la
     prendre en **volée indirecte**, et seulement
     - si la parabole lancée à 45° au plus passe à 2 m au-dessus du sol tout du long
       (`arc_clears`) : une crête collée au tireur arrête la montée, une crête collée devant la cible
       laisse un angle mort ;
     - et si un régiment allié (ni en déroute ni synthétique) voit la cible à 350 m au plus (portée
       réduite par la météo comme celle du tir) → précision × **0,3** ; sinon, si la cible a été vue
       par l'ennemi (tir direct ou guidé) il y a moins de **12 s** → précision × **0,2** ; sinon, pas
       de tir ;
   - les arbalètes (y compris génoises, malgré leur capacité `volley`), couleuvrines et javelots
     doivent voir leur cible ; les engins de siège sont inchangés.
   - `Unit::seen_at` retient l'instant où la cible a été vue ; `ShotEvent::indirect` et
     `get_shots()[].indirect` signalent au rendu une volée en cloche (rendu inchangé pour l'instant).
2. **Score de position unique** (`core/crates/sim-battle/src/position.rs`, `score_position`), en
   points « mètres de hauteur » comme la recherche de R2b :
   - hauteur au-dessus du déploiement + ½ proéminence − escarpement (R2b) ; glacis 0,3 × montée sur
     100 m (≤ 15 m) ;
   - contre-pente disponible derrière le front : + 3 (si l'ennemi a des tireurs) ;
   - couvert juste devant le front (à 14 m) : haie 1, fossé 0,8, clôture 0,45 × **14** points × part
     du front couverte ; village : 14 ;
   - champ de tir : 6 × part du terrain vu à 60, 120 et 180 m devant le front (une crête arrondie
     laisse un angle mort sous elle ; les archers doivent voir ce qu'ils tirent) ;
   - flancs appuyés (bois, rivière ou mare, marais, pente > 0,35) : 4 par aile à 30 m au plus de son
     extrémité, 2 jusqu'à 60 m. Les « bois de lisière » comptent ici, comme appuis d'aile (Crécy,
     Azincourt), pas comme couvert où poster les archers.
   - La position défensive (B6 et R2b réunis, `defensive_ground` dans `ai.rs`) compare le point de
     déploiement (+ 2 de marge), la grille des hauteurs de R2b (sans couvert, − 0,01/m latéral) et les
     couverts éligibles de B6 (− 0,03/m latéral, − 0,035/m en profondeur) : une haie sur une crête bat
     une crête nue et une haie en creux ; les archers vont derrière la haie de la crête.
   - Une position que l'ennemi atteindrait presque aussi vite est écartée (« course » : nos tireurs,
     au pas du plus lent et ralentis en montée, doivent y être en moins de 0,6 fois le temps de
     l'infanterie ennemie, depuis les lignes de déploiement) : sinon la ligne est prise en marche.
   - Derrière une haie sur une crête, les tireurs se collent à la haie (7, 5,5 ou 4,5 m) pour voir le
     glacis (la position qui voit le plus de terrain de 10 à 150 m devant la haie) ; derrière haie,
     fossé ou maisons, ils ne reculent devant l'infanterie qu'à 20 m (au lieu de 70 m), puisqu'elle
     doit franchir l'obstacle.
   - **Crête militaire** (`position::military_crest`) : sur une crête nue, les tireurs ne se postent
     pas au sommet topographique d'une croupe arrondie, qui laisse un angle mort sous lui, mais au
     point le plus haut du versant avant (pas de 5 m, 80 m au plus, pente ≤ 0,3) d'où au plus 10 %
     du glacis (de 30 à 180 m devant, trois files) est masqué (`dead_ground`) ; jamais en arrière de
     leur poste habituel devant la ligne. Le score de position d'une hauteur prend le champ de tir de
     sa crête militaire. Ils y vont au pas de course quand l'ennemi est à moins de 400 m, et n'y
     reviennent pas en tirant d'ailleurs. Une crête qui voit déjà son glacis ne change rien.
   - Sur la crête militaire, derrière leurs pieux plantés, les archers ne reculent plus devant des
     cavaliers de front (leur charge se brise sur les pieux) et ne reculent devant l'infanterie qu'à
     20 m : reculer derrière la ligne les mettrait hors de vue du glacis.
   - Une ligne défensive vient au secours de ses tireurs pris en mêlée à moins de 90 m.
   - Un défenseur à égalité de forces (ou dont les tireurs font plus de la moitié de la puissance)
     attend sur sa position un ennemi qui marche sur lui à moins de 250 m, au lieu de la quitter dès
     que ses archers ont éclairci les rangs adverses.
   - La ligne recule sur la contre-pente dès que l'ennemi a des tireurs, arcs longs compris.
   - Les tireurs ne s'arrêtent « à portée » que s'ils peuvent tirer ; ils cherchent un poste d'où ils
     voient la cible (le tir en cloche reste un pis-aller).
   - L'attaquant évalue le même score à l'emplacement de chaque régiment ennemi pour choisir qui
     frapper : 1,5 m d'écart latéral accepté par point de position en moins (`WEAK_POINT`).
3. **Attaquant plus haut** : un attaquant pas nettement plus fort (rapport < 1,25), plus haut de 6 m
   en moyenne qu'un ennemi dont les tireurs font plus de la moitié de la puissance, et qui ne perd pas
   le duel de tir, tient sa hauteur (posture défensive, ligne sur place) au plus **150 s**, puis
   attaque : pas de bataille figée.

## Mesures

Toutes les mesures sont déterministes (seules les victoires et pertes comptent).

**Non-régression R2b** (IA active contre camp passif, armées miroir, graines 0-63 × 2 camps) :

| Terrain | main afd327d4 | R4 |
|---|---|---|
| plaine | 113/128 | 114/128 |
| bocage | 89/128 | 88/128 |
| collines | 98/128 | 102/128 |
| montagne | 82/128 | 97/128 |
| total | 382/512 | 401/512 (399/512 après la crête militaire : 114, 88, 104, 93) |

**Contre-pente contre l'arc long** (`survey_reverse_slope_against_longbows`, 32 graines : défenseur
2 hommes d'armes + 1 arc long sur une crête, attaquant 3 arcs longs + 3 fantassins) : pertes au trait
de la ligne du défenseur **avant le contact** 88,1 → 17,4 par bataille, puis **2,3** avec la crête
militaire (− 97 %). Après le contact, 34,5 contre 13,2 (les archers anglais tirent en cloche, guidés,
sur la ligne pendant la mêlée) ; le résultat ne change pas (l'attaquant, deux fois plus fort, gagne
toujours).

**Anglais archers derrière une haie sur une crête contre chevaliers français**
(`survey_english_position_against_knights`, 32 graines ; 3 arcs longs + 2 hommes d'armes contre
2 chevaliers + 1 homme d'armes + 1 arbalète ; crête à 45 m devant le déploiement anglais) :

| Terrain | avant R4 : défenseur gagne / pertes françaises au trait | R4 sans crête militaire | R4 final |
|---|---|---|---|
| crête + haie | 32/32 · 300,0 | 32/32 · 300,0 | 32/32 · 300,0 |
| crête nue | 32/32 · 300,0 | **0/32** · 168,0 | **32/32** · 300,0 |
| haie en creux + crête | 32/32 · 300,0 | 32/32 · 145,1 | 32/32 · 300,0 |
| rase campagne | 32/32 · 274,9 | 32/32 · 247,8 | 32/32 · 266,5 |

(300 = armée française détruite par les flèches.) Sans crête militaire, les archers tenaient le sommet
d'une croupe arrondie et les chevaliers montaient à couvert dans l'angle mort : c'était un défaut de
placement, corrigé.

Rapport de forces serré (`R4_FRENCH=heavy` : 3 chevaliers + 1 homme d'armes + 1 arbalète) :

| Terrain | avant R4 | R4 final |
|---|---|---|
| crête + haie | 32/32 · ≈ 306 | 0/32 · 152 |
| crête nue | 32/32 · ≈ 302 | 0/32 · 242 (la bataille dure 678 s au lieu de 262 s) |
| haie en creux + crête | 32/32 · ≈ 301 | 0/32 · 206 |
| rase campagne | 0/32 · 238 | 0/32 · 177 |

À ce rapport de forces, les Anglais gagnaient avant R4 **grâce au tir à l'aveugle** : dès leur
déploiement derrière la crête, leurs volées traversaient le relief à pleine précision sur des
chevaliers qu'ils ne voyaient pas, puis continuaient depuis l'arrière de leur ligne. Avec R4, ils
doivent d'abord monter à la crête (≈ 60 à 90 s) avant de tirer, et la haie, qui couvre aussi les
chevaliers arrêtés juste devant elle (couvert B5 dans les 14 m de la cible), réduit les dernières
volées ; les chevaliers arrivent au contact avec la moitié de leurs hommes et font céder des archers
au moral bas (55). Pistes essayées sans effet sur l'issue : archers collés ou reculés derrière la
haie, repli devant l'infanterie à 20 m, pas de course vers la haie (il retardait le contact de la
démo en bocage : écarté), ligne qui vient secourir ses archers, ligne qui ne se cache plus devant
une seule arbalète. Faire pencher ce cas vers les Anglais demanderait de toucher à l'équilibre
(moral des archers, efficacité des flèches sur les chevaliers, portée du couvert de haie), hors du
périmètre de R4.

## Conséquences

- Contre une armée d'archers, une ligne derrière une crête ne subit plus que des volées guidées et
  dispersées ; un observateur sur la crête (cavalerie d'aile, tireurs en avant) redevient utile.
- Les arbalétriers génois perdent le tir à travers le relief qu'ils tenaient de leur capacité
  `volley` : un carreau ne se tire pas en cloche au-dessus d'une crête ; la capacité ne sert plus
  qu'au tir indirect des troupes qui tirent des flèches.
- Les batailles de campagne avec site changent (position défensive et choix de cible) ; mêmes graines,
  mêmes tirages.
- Rendu : une volée `indirect` est dessinée en cloche 2,5 fois plus haute et plus longue
  (`battle_volley.gdshaderinc` et `BattleVolleys.arrow_landing`, bit 32 du code du paquet ; le pas
  des traits confiés à la couche plantée passe au bit 64).
- Point ouvert : à rapport de forces serré, la position anglaise perd (voir Mesures) ; question
  d'équilibre des unités. **Traité par l'ADR 0052** (panique des chevaux sous les traits, haie tenue
  par les tireurs) : crête + haie 8 → 20/32 (mesure à crête décalée), rase campagne toujours perdue.

## Suite SG4 (2026-09-25) : avantage de la hauteur en mêlée, cavalerie qui couvre ses tireurs

Mesure (`sim-battle/tests/sg4_balance.rs`, ignoré : 60 régiments × 120 hommes par camp, armées
miroir, IA des deux côtés, graines 1-10 ; crête gaussienne de 20 m à 45 m devant le défenseur ;
« sans pieux » retire la capacité `stakes`) — victoires attaquant / défenseur / nuls :

| Terrain | Pieux | main (après ADR 0052) | SG4 |
|---|---|---|---|
| plat | oui | 7 / 3 / 0 | 3 / 7 / 0 |
| plat | non | **2 / 8** / 0 | 9 / 1 / 0 |
| crête | oui | 0 / 10 / 0 | 0 / 10 / 0 |
| crête | non | 0 / 10 / 0 | 0 / 10 / 0 |
| plaine générée, sans village | oui | 4 / 5 / 1 | 5 / 5 / 0 |
| plaine générée, sans village | non | 3 / 6 / 1 | 4 / 4 / 2 |
| comme `ep1_scale` (village, météo tirés) | oui | 4 / 5 / 1 | 4 / 6 / 0 |

(Avant la fusion de l'ADR 0052, main donnait plat 5/5, 0/10 ; crête 0/8 + 2 nuls, 0/10 ; plaine
générée 3/7, 4/4 + 2 nuls.) Hors crête, l'attaquant gagne 25 batailles sur 50 avec SG4 (20/50 sur
main) ; sur la crête, le défenseur gagne toujours.

Diagnostic : l'issue se joue dans le duel de cavalerie. Les tireurs de l'attaquant, qui avancent
pour tirer, se retrouvent « isolés » devant leur ligne ; la cavalerie du défenseur les charge et la
cavalerie de l'attaquant ne réagissait qu'à des cavaliers à moins de 160 m d'elle-même (le
commentaire annonçait « ou de nos tireurs »). Sur un terrain plat sans pieux, l'attaquant perdait
ainsi 8 fois sur 10 à forces égales. Par ailleurs, rien dans la mêlée ne tenait compte de la pente.

Décision :
1. **Avantage de la hauteur en mêlée**, en données (`data/rules/battle_crest.json`, schéma
   `battle_crest_rules.schema.json`, `crest::CrestRules`) : un régiment plus haut que son adversaire
   de plus de `min_height_m` (1,5 m) frappe `melee_per_m` (5 %) plus fort par mètre au-delà, jusqu'à
   `max_height_m` (6 m, soit ± 22,5 %) ; en montée, autant de moins. Les plis du terrain (sous 1,5 m)
   ne comptent pas : une bataille de plaine n'en dépend pas. Un facteur de plus dans
   `melee_damage` (hauteur du sol au centre des deux régiments), sans toucher au moral ni à
   l'engagement, ni à la signature des modificateurs de terrain.
2. **La cavalerie couvre ses tireurs** (`plan_horse`, règle 1) : hors posture défensive, elle
   contre-charge aussi les cavaliers ennemis qui arrivent à moins de `SHOOTER_GUARD` (160 m) d'un de
   ses tireurs, à portée de cavalerie (450 m). Une armée en défense garde sa cavalerie sur sa
   position (sinon la crête perd son avantage : 7/3 pour l'attaquant sur crête sans pieux).
3. Les pieux et la crête militaire de R4 sont inchangés : la mesure montre que les tireurs derrière
   leurs pieux sur la crête ne décident pas de l'issue (seuil de repli 20 → 45 m, portée de la crête
   militaire 80 → 40 m : résultats identiques).

Conséquences : empreintes `b6.rs` (graines 3 et 11) recalculées, mêmes vainqueurs. `ep1_scale`
(graine 11) : pic de 10 régiments en mêlée (16 et 26 aux graines 3 et 5, 41-47 aux graines 1, 2, 4) ;
le test compte aussi les régiments qui ont combattu au corps à corps au moins une fois (55 à 95 sur
120) et exige au moins 10 au pic et 40 en tout.

Après fusion d'EP9 (ADR 0056, armée brisée à 40 % d'effectif en état de combattre ; batailles de
5-6 min) — victoires attaquant / défenseur, 10 graines, main EP9 seul → EP9 + SG4 :
plat avec pieux 3/7 → 3/7 ; plat sans pieux **0/10 → 0/10** ; crête 0/10 → 0/10 (avec et sans
pieux) ; plaine générée 2/8 → 2/8 et 6/4 → 5/5 ; comme `ep1_scale` 3/7 → 2/8. SG4 ne change plus
l'issue : l'armée cède avant le duel de cavalerie et la mêlée. Sur le plat sans pieux, la milice de
l'attaquant (moral 40) traverse 180 m de flèches après `ATTACKER_DUEL_LIMIT` (180 s, EP9) et se
débande (moral 14 à 300 s) alors que ses tireurs gagnaient le duel ; son armée est brisée. Point
ouvert pour EP9 (logique hors de SG4), traité par EP9b (ADR 0056 § EP9b : plat sans pieux 0/10 → 7/3, crête avec pieux 0/10 → 6/4). `b6.rs` : empreintes recalculées sur EP9 + SG4 (mêmes
vainqueurs) ; `ep1_scale` : pic 10-19 en mêlée, 28-59 régiments au contact (seuil 25).
