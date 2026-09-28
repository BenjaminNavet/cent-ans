# CB4 — Capacités actives : relecture historique

Relecture de l'historien, 27 septembre 2026, avant tout code du lot CB4.
Références de travail : spec `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`
(§ CB4), plan `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ CB4), ordres de
chef `data/battle_orders/`, types d'unité `data/unit_types/`, fiches Codex `cdx_jeu_pieux`,
`cdx_jeu_formations`, `cdx_pavois`.

## Principes retenus

1. **Une capacité = un geste de troupe attesté**, que des soldats ordinaires faisaient sur ordre
   de leur capitaine : se couvrir, se serrer, planter, changer de tir. Rien qui ne tienne qu'à un
   individu exceptionnel (duel, cri qui terrifie, « inspiration ») : la spec l'exclut, et ce que
   le chef peut faire est déjà couvert par les ordres de chef (cri de guerre, rallier, pied à
   terre, pas de quartier).
2. **Un coût visible** pour chaque avantage : immobilité, vitesse, portée, cohésion, flancs. C'est
   aussi la leçon des sources : chaque parade de l'époque a son point faible (pieux à Patay,
   schiltron sous les flèches à Falkirk et Halidon Hill, masse serrée à Roosebeke et Azincourt).
3. **Pas de doublon** avec les formations (touche T : ligne, colonne, schiltron, coin), les modes
   de CB2 (garde, escarmouche, course, mêlée) ni les capacités passives existantes (`stakes`,
   `volley`, `pike_square`, `charge_lance`, `rain_penalty`).
4. **Effets en ordre de grandeur** seulement ; les chiffres fins relèvent de l'équilibrage (EP7/EQ7).

## Tableau de synthèse

| # | Capacité proposée (spec) | Verdict | Nom recommandé | Unités |
|---|---|---|---|---|
| 1 | Tir tendu (archers anglais) | **garder**, effet nuancé | Tir tendu | `unit_longbowmen`, `unit_mounted_archers` (à pied), `unit_francs_archers` |
| 2 | Derrière le pavois (arbalétriers) | **garder** (reprend l'ordre `order_pavise`), + délai de pose et arc frontal | Dresser les pavois | `unit_crossbowmen`, `unit_genoese_crossbowmen`, `unit_gascon_crossbowmen`, `unit_culveriners` |
| 3 | Charge en haie (chevaliers) | **modifier** : doublon du coin et de la charge, terme tardif | Se rallier à la bannière | `unit_knights`, `unit_breton_knights`, `unit_ordonnance_gendarmes`, `unit_mounted_sergeants` |
| 4 | Rangs serrés (hommes d'armes à pied) | **garder**, renommer (pas de « mur de boucliers ») | Serrer les rangs | `unit_men_at_arms_foot`, `unit_english_retinue`, `unit_routiers`, milices (`unit_urban_militia`, `unit_goedendag_militia`, `unit_welsh_spearmen`) |
| 5 | Hérisson (piquiers) | **modifier** : doublon du schiltron, terme suisse du XVe s. | Piques plantées | `unit_flemish_pikemen`, `unit_scottish_spearmen`, `unit_goedendag_militia` |
| 6 | Tir de rupture (engins) | **modifier** : mode persistant plutôt que capacité à recharge | Battre en brèche | `unit_trebuchet`, `unit_mangonel`, `unit_bombard` |
| 7 | (ajout) Planter les pieux | **à décider** : passif actuel à garder, mais borné dans le temps | Planter les pieux | `unit_longbowmen` |

Capacités examinées et **écartées** : flèches enflammées en rase campagne, « mur de boucliers »,
défi et duel de champions, salve de terreur, fuite simulée générique (voir § 8).

---

## 1. Tir tendu (archers)

**Pratique attestée : oui, sous une autre forme que la légende.** Les archers anglais tirent en
cloche à longue distance (la « grêle » ou « neige » de flèches des chroniqueurs : Froissart à
Crécy, le Religieux de Saint-Denis et Monstrelet à Azincourt), puis de plus en plus à plat quand
l'ennemi approche. Le tir à courte distance, visé, sur les chevaux et les défauts de l'armure, est
décrit explicitement :

- **Poitiers, 1356** — Geoffrey le Baker (*Chronicon*, éd. E. M. Thompson, 1889 ; trad. Preest et
  Barber, 2012) : les archers de l'aile du comte de Salisbury ne font rien contre les chevaux
  bardés de front ; le comte d'Oxford les mène sur le flanc pour tirer dans l'arrière-train des
  chevaux, qui se renversent. C'est le meilleur texte sur un tir ajusté, choisi par le capitaine.
- **Azincourt, 1415** — *Gesta Henrici Quinti* (éd. Taylor et Roskell, 1975), Monstrelet
  (*Chronique*, éd. Douët-d'Arcq, SHF, t. III), Jean Le Fèvre de Saint-Rémy et Jean de Waurin :
  les hommes d'armes français avancent tête baissée, visière close, sous le tir, qui frappe
  surtout les chevaux des ailes et fait refluer les montures blessées ; puis les archers tirent de
  flanc dans la masse.

**Débat sur l'efficacité** (à connaître pour régler l'effet) :
- Clifford J. Rogers, « The Efficacy of the English Longbow : A Reply to Kelly DeVries », *War in
  History* 5 (1998) : le tir rapproché perce la maille et les défauts de l'armure ; l'arc est
  décisif par le nombre et la cadence.
- Kelly DeVries, *Infantry Warfare in the Early Fourteenth Century* (1996) et « Catapults are not
  Atomic Bombs », *War in History* 4 (1997) : l'arc tue surtout des chevaux et désorganise ; la
  décision se fait en mêlée.
- Alan Williams, *The Knight and the Blast Furnace* (2003) ; Strickland et Hardy, *The Great
  Warbow* (2005) : le harnois blanc de bonne qualité (après 1400) résiste à la flèche de face,
  sauf aux défauts ; la maille et le gambison beaucoup moins.

**Période** : toute la période (1337-1453) ; plus efficace avant 1400 (maille, plates partielles)
qu'après (harnois complet des hommes d'armes nobles), ce que le jeu peut laisser à l'armure.

**Unités** : `unit_longbowmen` ; `unit_francs_archers` (après 1448) ; `unit_mounted_archers`
seulement à pied (ils tiraient à pied). **Pas les arbalétriers** : l'arbalète tire déjà à plat,
c'est sa nature, et sa cadence ne change pas.

**Effet de jeu plausible** :
- portée nettement réduite (de l'ordre d'un tiers à la moitié : tir visé utile à 50-100 m contre
  200-250 m en cloche, ordre de grandeur de Strickland et Hardy) ;
- précision nettement accrue ; bonus **surtout contre les chevaux et les troupes peu protégées** ;
  pénétration en légère hausse seulement contre les hommes en harnois (pas de « perce-armure ») ;
- cadence un peu plus lente (on vise) ou inchangée ; consommation de flèches identique.
- Le bonus de flanc existant (tir de côté) doit rester le vrai multiplicateur, comme à Poitiers.

**Conditions** : munitions > 0 ; pas au contact ; cible dans la portée réduite. Se désactive au
contact ou si les munitions tombent à 0.

**IA** : quand l'ennemi le plus proche est à moins de la portée réduite, surtout s'il est monté.

**Risques** : le nom « tir tendu » est un terme moderne d'artillerie ; acceptable en UI (clair
pour le joueur). Ne pas en faire un tir « perforant » qui annulerait l'armure : ce serait la
légende de l'arc qui transperce tout, que les historiens ont nuancée.

**Verdict : garder**, avec un effet orienté « précision et chevaux » plutôt que « pénétration ».

## 2. Dresser les pavois (arbalétriers) — reprise de l'ordre `order_pavise`

**Pratique attestée : oui, abondamment.** Le grand pavois (*pavese*) des arbalétriers italiens,
porté par un pavoisier ou par le tireur, planté au sol par une pointe, derrière lequel on
retend l'arbalète. Les milices urbaines françaises et flamandes l'emploient aussi.

- **Crécy, 1346** : les Génois engagent le combat sans leurs pavois, restés aux bagages, après une
  longue marche, et sont décimés par les archers. Le fait est rapporté par des chroniques
  françaises (Grandes Chroniques de France, *à vérifier sur l'édition*) et retenu par Jonathan
  Sumption (*Trial by Battle*, 1990), DeVries (1996) et Ayton et Preston (*The Battle of Crécy,
  1346*, 2005). Jean le Bel et Froissart insistent, eux, sur l'averse et les cordes mouillées.
- Valérie Serdon, *Armes du diable. Arcs et arbalètes au Moyen Âge* (2005) : pavois et
  pavoisiers dans les compagnies d'arbalétriers ; Philippe Contamine, *Guerre, État et société à
  la fin du Moyen Âge* (1972) : pavois dans l'armement des milices et des arbalétriers royaux.
- Couleuvriniers : les pavois abritent aussi les premières armes à feu portatives (Hussites des
  années 1420, ordonnances bourguignonnes des années 1470 ; cf. fiche `cdx_wagenburg`). Leur
  capacité `pavise` actuelle est donc juste.

**Période** : toute la période.

**Unités** : `unit_crossbowmen`, `unit_genoese_crossbowmen`, `unit_gascon_crossbowmen`,
`unit_culveriners` (tous ont déjà `pavise`). Pas les archers (l'arc long se tire debout,
découvert ; l'archer anglais n'avait pas de pavois).

**Effet de jeu** : garder le facteur actuel de dégâts de missile (0,35) — ordre de grandeur juste :
le pavois arrête l'essentiel des flèches de face, pas toutes.

**Conditions (à ajouter par rapport à l'ordre actuel)** :
- **immobile** ; les pavois sont levés au premier mouvement (déjà le cas) ;
- **délai de pose** de quelques secondes avant que la protection compte (même logique que les
  pieux : une troupe surprise en marche en est dépourvue — c'est tout Crécy) ;
- **protection de face seulement** (arc frontal) : de flanc ou de dos, le pavois ne couvre rien ;
- inutile au contact : un pavois n'est pas un bouclier de mêlée (pas de bonus en mêlée).

**IA** : dès qu'un tireur ennemi a la ligne de vue et que l'unité est à l'arrêt pour tirer (règle
actuelle `under_fire_within` suffit).

**Risque** : aucun anachronisme. Ne pas l'étendre à toute l'infanterie (les milices avaient des
pavois, mais l'effet de jeu doit rester propre aux tireurs, sinon il devient un « mur de
boucliers » bis).

**Verdict : garder**, en retirant `order_pavise.json` des ordres de chef comme prévu ; reprendre
son texte de description (Crécy) dans l'infobulle de la capacité.

## 3. Charge en haie (chevaliers) → « Se rallier à la bannière »

**Ce que dit l'histoire.**
- Les cavaliers du XIVe siècle chargent par **conrois** (petites unités serrées derrière une
  bannière ou un pennon), genou contre genou, en plusieurs échelons, et non en une seule ligne
  mince. J. F. Verbruggen, *The Art of Warfare in Western Europe during the Middle Ages* (2e éd.,
  1997), et Contamine, *La Guerre au Moyen Âge* (1980), sur la charge en ordre serré et le
  ralliement à la bannière.
- L'expression **« en haie »** (une seule ligne de lances, sans profondeur) désigne surtout la
  gendarmerie de la fin du XVe et du XVIe siècle (compagnies d'ordonnance, guerres d'Italie).
  Pour 1337-1445, elle est **anachronique** ; elle ne conviendrait, au mieux, qu'aux
  `unit_ordonnance_gendarmes` (après 1445).
- Le jeu a déjà : le bonus de charge à pleine vitesse (CB2 : « pas de bouton »), la formation
  **coin** (bonus de charge ×1,2, trois rangs de front) et la capacité passive `charge_lance`.
  Une capacité « charge + » ferait doublon, et un bonus de charge à la demande glisse vers la
  capacité de héros.

**Ce qui manque vraiment** : le problème tactique majeur de la chevalerie à l'époque est de **se
reformer après le choc** au lieu de poursuivre ou de se disperser. Les chroniqueurs louent les
capitaines qui « se recueillent » à leur bannière et rechargent (Froissart, *passim* ; la *Vie du
Prince Noir* du héraut Chandos, éd. Tyson, 1975, pour Nájera). Les chevaliers qui s'égaillent
perdent la bataille : charges françaises successives et désordonnées à Crécy (Jean le Bel,
*Chronique*, éd. Viard et Déprez, SHF, 1904-1905, t. II ; Froissart).

**Proposition : « Se rallier à la bannière »**
- **Effet** : l'unité s'arrête et se reforme : récupération de cohésion nettement accélérée,
  petite remontée de moral ; pendant la durée, vitesse nulle ou très faible et aucune charge.
- **Conditions** : pas au contact ; pas en déroute (ce n'est pas le ralliement de fuyards, qui
  reste l'ordre de chef « Rallier ») ; utile surtout après une charge.
- **Unités** : cavalerie lourde (`unit_knights`, `unit_breton_knights`,
  `unit_ordonnance_gendarmes`, `unit_mounted_sergeants`) ; pas les cavaliers légers.
- **IA** : après une charge, dès que l'unité n'est plus au contact et que sa cohésion est basse.
- **Risque** : aucun anachronisme ; effet défensif et coûteux (immobilité), donc pas une capacité
  de héros.

**Verdict : modifier** (remplacer « Charge en haie » par « Se rallier à la bannière »). Si le
joueur tient à une charge « en haie », la réserver aux gendarmes d'ordonnance et en faire une
variante de formation (front large, peu profond) plutôt qu'une capacité.

## 4. Rangs serrés (hommes d'armes à pied) → « Serrer les rangs »

**Pratique attestée : oui.** Les hommes d'armes démontés combattent en masse compacte, lance
raccourcie :
- **Poitiers, 1356** : les Français retaillent leurs lances à cinq pieds et ôtent leurs éperons
  avant de combattre à pied (Froissart, *Chroniques*, éd. Luce, SHF, t. V).
- **Azincourt, 1415** : la masse française est si serrée que les hommes ne peuvent lever leurs
  armes (Monstrelet ; *Chronique du Religieux de Saint-Denys*, éd. Bellaguet ; Anne Curry, *The
  Battle of Agincourt : Sources and Interpretations*, 2000). John Keegan, *The Face of Battle*
  (1976), analyse l'écrasement.
- **Roosebeke, 1382** : les Flamands, trop serrés, s'étouffent dans leur propre masse quand les
  ailes françaises les pressent (Froissart ; fiche `cdx_roosebeke`).
- Milices : la cohésion serrée fait la force des communes flamandes (Courtrai, 1302 ; Verbruggen,
  *The Battle of the Golden Spurs*, trad. 2002) et des lanciers.

**Nom** : pas de **« mur de boucliers »** en UI : au XIVe siècle l'écu se réduit, et vers 1400
l'homme d'armes en harnois combat souvent sans bouclier, à deux mains (hache, bec de faucon,
lance raccourcie). L'identifiant interne `ShieldWall` peut rester ; le libellé sera « Serrer les
rangs ».

**Effet de jeu** :
- défense frontale et résistance à la poussée et à la charge en hausse modérée ;
- vitesse nettement réduite ; pas de charge ;
- **coûts attestés** : vulnérabilité accrue de flanc et de dos, légère hausse des pertes sous le
  tir (cible dense), fatigue un peu plus rapide en mêlée prolongée (Azincourt).

**Conditions** : pas d'immobilité exigée (on avance serré), mais pas de course.

**Unités** : `unit_men_at_arms_foot`, `unit_english_retinue`, `unit_routiers`,
`unit_urban_militia`, `unit_goedendag_militia`, `unit_welsh_spearmen` (tous ont `shield_wall`),
et les chevaliers une fois pied à terre.

**IA** : en posture défensive au contact frontal, ou à l'approche d'une charge montée si l'unité
n'a pas de piques.

**Verdict : garder**, renommer « Serrer les rangs », avec le coût de flanc et de tir.

## 5. Hérisson (piquiers) → « Piques plantées »

**Constat.**
- Le « hérisson » tous azimuts **existe déjà** : c'est la formation schiltron (touche T,
  `Formation::Square`, capacité `pike_square` : sans flanc, piquiers brisant toute charge montée).
  Une capacité identique ferait doublon.
- Le mot **« hérisson »** (all. *Igel*) désigne la formation suisse du XVe siècle (Arbedo 1422,
  Saint-Jacques-sur-la-Birse 1444) : étranger à nos Flamands et Écossais du XIVe siècle.

**Ce qui manque** : le geste de **recevoir la charge de front**, talon de la pique ou du
goedendag calé au sol, pointe à hauteur de poitrail, sans quitter la ligne. Attesté à Courtrai
(1302, Flamands ; Verbruggen, 2002), à Bannockburn (1314, schiltrons écossais) et à Roosebeke
(1382). DeVries (*Infantry Warfare*, 1996) consacre son livre à ces infanteries.

**Proposition : « Piques plantées »**
- **Effet** : bonus fort contre la cavalerie **de face seulement** (la charge frontale perd presque
  tout effet de choc, pertes accrues pour les chevaux) ; ne protège ni les flancs ni le dos
  (c'est le schiltron qui le fait, au prix de la lenteur et de la vulnérabilité au tir).
- **Conditions** : immobile ; pas encore au contact ; délai de pose très court.
- **Unités** : `unit_flemish_pikemen`, `unit_scottish_spearmen`, `unit_goedendag_militia`.
- **IA** : cavalerie ennemie en approche de face à moins de 100-150 m.

**Verdict : modifier** (« Piques plantées », frontal ; le hérisson tous azimuts reste la formation
schiltron).

## 6. Tir de rupture (engins) → « Battre en brèche »

**Pratique attestée : oui.** Deux usages des engins :
- **battre la muraille** en concentrant les coups sur un même point du pied d'un mur ou d'une tour
  pour ouvrir la brèche : engins devant Aiguillon (1346) et Calais (1346-1347) (Froissart ;
  Sumption, *Trial by Battle*), bombardes à Harfleur en 1415 (*Gesta Henrici Quinti*), et surtout
  l'artillerie des frères Bureau en Normandie en 1449-1450 (Contamine, 1972) ;
- **tirer par-dessus**, dans la ville ou sur les défenseurs du chemin de ronde, pour harceler.
Référence technique : Kelly DeVries et Robert D. Smith, *Medieval Military Technology* (2e éd.,
2012).

**Le terme juste** est « battre en brèche », plus parlant que « tir de rupture ».

**Effet de jeu** : dégâts aux murs et aux portes en hausse nette ; cadence un peu plus lente
(visée, recalage) ; **aucun effet sur les hommes** pendant le mode. Les engins sont déjà très
lents (une grosse bombarde tire quelques coups à l'heure) : l'effet porte sur les dégâts.

**Forme** : c'est un **choix de cible durable**, pas un coup de fouet temporaire. Recommandation :
un **mode persistant** (bascule, sans durée ni recharge) plutôt qu'une capacité à recharge.

**Conditions** : siège seulement ; immobile (en batterie) ; mur ou porte à portée.

**Unités** : `unit_trebuchet`, `unit_bombard` ; `unit_mangonel` avec un effet moindre (engin plus
léger, plutôt antipersonnel). Pas la tour de siège.

**IA** : mode actif par défaut chez l'assiégeant tant qu'aucune brèche n'est ouverte.

**Verdict : modifier** (renommer « Battre en brèche », mode persistant).

## 7. Ajout à trancher : « Planter les pieux »

La capacité passive `stakes` existe déjà : 15 s d'immobilité, puis protection frontale (fiche
`cdx_jeu_pieux`). C'est juste historiquement (Azincourt 1415 : ordre d'Henri V de tailler les
pieux, *Gesta Henrici Quinti* ; Patay 1429 : archers de Talbot surpris avant d'avoir fini de
planter, témoignage de Jean de Waurin, présent du côté anglais).

Deux points à trancher :
1. **Passif ou actif ?** Le passif automatique est bon et lisible ; un bouton donnerait au joueur
   le choix (et le risque) du moment, comme à Patay. Recommandation : **garder le passif**, et
   afficher l'état « pieux plantés » avec les icônes d'état de CB2 plutôt qu'un bouton.
2. **Date.** Les pieux d'archers ne sont attestés chez les Anglais qu'à partir de 1415 (chez les
   Ottomans à Nicopolis en 1396). À Crécy, on signale au mieux des trous creusés devant le front
   (Geoffrey le Baker, *à vérifier sur l'éd. Thompson*) ; à Poitiers, haies, fossés et
   chariots. Or `unit_longbowmen` a `stakes` dès 1337 : **anachronique pour 1337-1414**.
   Proposition : conditionner `stakes` à une technologie ou à une date (vers 1400-1415).

## 8. Capacités examinées et écartées

| Idée | Pourquoi l'écarter |
|---|---|
| Flèches enflammées (archers) | Attestées aux sièges (toits de chaume), pas en rase campagne contre des hommes ; le système d'incendie (S2) couvre déjà les sièges. |
| « Mur de boucliers » | Terme et pratique du haut Moyen Âge ; remplacé par « Serrer les rangs ». |
| Défi, duel de champions, cri qui terrifie | Capacités de héros exclues par la spec ; le défi existe (Combat des Trente, 1351) mais hors bataille rangée, et le duel est déjà un système à part (`battle_duel.json`). |
| Fuite simulée générique | Rare et risquée au XIVe siècle ; le *tornafuye* des jinetes castillans est couvert par le mode escarmouche (CB2). |
| Salve au signal (Erpingham à Azincourt) | Geste de chef, non d'unité ; le tir à volonté (F) et `volley` suffisent. |
| Pied à terre en capacité d'unité | La décision de démonter est prise par le chef avant la bataille (Crécy, Poitiers, Cocherel) : elle reste un ordre de chef. |

## Points qui demandent une décision de jeu

1. **Charge en haie** : remplacer par « Se rallier à la bannière » (recommandé), ou la réserver aux
   gendarmes d'ordonnance comme variante de formation.
2. **Hérisson** : remplacer par « Piques plantées » frontal (recommandé), le hérisson restant la
   formation schiltron.
3. **Battre en brèche** : mode persistant (recommandé) ou capacité à recharge comme les autres.
4. **Pavois** : ajouter un délai de pose et un arc frontal (recommandé) ; facteur 0,35 gardé.
5. **Pieux** : garder le passif ; décider d'une borne de date ou de technologie (vers 1400-1415).
6. **Tir tendu** : effet surtout précision et chevaux ; pas un perce-armure.

## Sources

**Chroniqueurs**
- Jean le Bel, *Chronique*, éd. J. Viard et E. Déprez, SHF, 2 vol., 1904-1905.
- Jean Froissart, *Chroniques*, éd. S. Luce et al., SHF, t. III (Crécy) et t. V (Poitiers).
- Geoffrey le Baker, *Chronicon*, éd. E. M. Thompson, Oxford, 1889 ; trad. D. Preest, intr.
  R. Barber, 2012.
- Héraut Chandos, *La Vie du Prince Noir*, éd. D. B. Tyson, 1975.
- *Gesta Henrici Quinti*, éd. F. Taylor et J. S. Roskell, Oxford, 1975.
- Enguerrand de Monstrelet, *Chronique*, éd. L. Douët-d'Arcq, SHF, t. III.
- Jean Le Fèvre de Saint-Rémy, *Chronique*, éd. F. Morand, SHF.
- Jean de Waurin, *Recueil des croniques*, éd. W. Hardy, Rolls Series.
- *Chronique du Religieux de Saint-Denys*, éd. L. Bellaguet.

**Historiens**
- Philippe Contamine, *Guerre, État et société à la fin du Moyen Âge*, 1972 ; *La Guerre au Moyen
  Âge*, 1980.
- Anne Curry, *The Battle of Agincourt : Sources and Interpretations*, 2000 ; *Agincourt : A New
  History*, 2005.
- Clifford J. Rogers, *War Cruel and Sharp*, 2000 ; « The Efficacy of the English Longbow »,
  *War in History* 5, 1998 ; (dir.) *The Wars of Edward III : Sources and Interpretations*, 1999.
- Kelly DeVries, *Infantry Warfare in the Early Fourteenth Century*, 1996 ; avec R. D. Smith,
  *Medieval Military Technology*, 2e éd., 2012.
- Jonathan Sumption, *The Hundred Years War*, I *Trial by Battle*, 1990 ; II *Trial by Fire*, 1999.
- Andrew Ayton et Philip Preston, *The Battle of Crécy, 1346*, 2005.
- Matthew Strickland et Robert Hardy, *The Great Warbow*, 2005.
- Alan Williams, *The Knight and the Blast Furnace*, 2003.
- J. F. Verbruggen, *The Art of Warfare in Western Europe during the Middle Ages*, 2e éd., 1997 ;
  *The Battle of the Golden Spurs*, trad. 2002.
- Valérie Serdon, *Armes du diable. Arcs et arbalètes au Moyen Âge*, 2005.
- John Keegan, *The Face of Battle*, 1976.

Les mentions « à vérifier » signalent un détail dont je n'ai pas l'édition sous les yeux ; elles
ne changent aucun verdict.
