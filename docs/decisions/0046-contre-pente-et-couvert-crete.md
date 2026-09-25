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
   - flancs appuyés (bois, rivière ou mare, marais, pente > 0,35) : 4 par aile à 30 m au plus de son
     extrémité, 2 jusqu'à 60 m. Les « bois de lisière » comptent ici, comme appuis d'aile (Crécy,
     Azincourt), pas comme couvert où poster les archers.
   - La position défensive (B6 et R2b réunis, `defensive_ground` dans `ai.rs`) compare le point de
     déploiement (+ 2 de marge), la grille des hauteurs de R2b (sans couvert, − 0,01/m latéral) et les
     couverts éligibles de B6 (− 0,03/m latéral, − 0,035/m en profondeur) : une haie sur une crête bat
     une crête nue et une haie en creux ; les archers vont derrière la haie de la crête.
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

Voir `docs/wip/r4-couvert-crete.md` (mesure R2b sur 512 batailles, surveys `tests/r4_survey.rs`).

## Conséquences

- Contre une armée d'archers, une ligne derrière une crête ne subit plus que des volées guidées et
  dispersées ; un observateur sur la crête (cavalerie d'aile, tireurs en avant) redevient utile.
- Les arbalétriers génois perdent le tir à travers le relief qu'ils tenaient de leur capacité
  `volley` : un carreau ne se tire pas en cloche au-dessus d'une crête ; la capacité ne sert plus
  qu'au tir indirect des troupes qui tirent des flèches.
- Les batailles de campagne avec site changent (position défensive et choix de cible) ; mêmes graines,
  mêmes tirages.
- Point ouvert : dessiner une cloche plus haute pour les volées `indirect` (le rendu BV1 encode
  l'arc par type de projectile dans le shader ; un bit de plus décalerait le codage des traits).
