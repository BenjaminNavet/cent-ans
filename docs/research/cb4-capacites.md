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
