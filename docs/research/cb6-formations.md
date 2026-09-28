# CB6 — Formations de groupe : relecture historique des préréglages

Relecture de l'historien, 27 septembre 2026, pour `data/rules/group_formations.json` (lot CB6,
plan `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`). Rôles du jeu : `infantry`,
`foot_ranged`, `cavalry` (y compris tireurs montés), `siege`, `general`.

## Vocabulaire de l'époque

- Une armée se range en **« batailles »** (*acies*, *battles*) : d'ordinaire trois, **avant-garde**
  (ou « première bataille »), **bataille** (celle du roi ou du chef) et
  **arrière-garde**, parfois une quatrième ou des **ailes** montées. C'est la même division qu'en
  marche. Contamine, *La Guerre au Moyen Âge* (1980) ; Verbruggen, *The Art of Warfare in Western
  Europe* (2e éd., 1997).
- « **Ordonner ses batailles** », « se mettre **en bataille** », « **en ordonnance** » : dire d'une
  armée rangée pour combattre. « Ligne de bataille » est plus moderne mais compris de tous.
- « **Cavalerie** » est un mot du XVIe siècle : dans les noms, préférer « chevalerie », « gens
  d'armes à cheval » ou « hommes d'armes montés ». Le mot peut rester dans les textes d'aide.
- Unités de distance des chroniqueurs : **« un trait d'arc »** (de l'ordre de 200 m) et « un jet
  de pierre ». Utile pour les écarts ci-dessous.

## Constat sur le placement actuel (`sim.rs`, `deploy`, vers la ligne 531)

Le code ne fait pas tout à fait ce que dit le plan : pour chaque camp, l'infanterie est sur la
ligne, **les tireurs à pied 45 m derrière elle** (et non devant), les cavaliers sur les ailes 15 m
en retrait, les engins 95 m derrière. Sans infanterie, les tireurs prennent la ligne. Pour la
non-régression, le préréglage « Ligne de bataille » doit reproduire ce placement ; mais le texte
du plan (« tireurs devant ») est faux pour le code, et c'est historiquement discutable (voir § 1).

## Tableau de synthèse

| Préréglage (plan) | Verdict | Nom recommandé | Posture |
|---|---|---|---|
| Ligne de bataille | **garder** (= placement actuel) ; décision sur la place des tireurs | Ligne de bataille | défense |
| Herse | **garder**, corriger la description (pas de « coins » d'archers entre les batailles) | La herse | défense |
| Trois batailles | **garder** | Trois batailles | attaque ou défense |
| Charge | **modifier** : échelonner la chevalerie avec une infanterie qui suit ; limiter la charge frontale | Charge de la chevalerie | attaque |
| Colonne | **modifier** : ordre par batailles, pas par vitesse ; vitesse du plus lent | Ordre de marche | marche |
| (manquant) | **ajouter** | Bataille à pied | défense |
| (manquant) | **à étudier** (exige un objet charroi) | Camp retranché | défense |

---

## 1. Ligne de bataille

**Historique.** Disposition générique, commune à toutes les armées : les gens de pied au centre en
une ou plusieurs batailles côte à côte, les montés sur les ailes, les engins derrière. La place
des tireurs varie :
- **devant** en ouverture de combat, le plus souvent : Génois devant les batailles françaises à
  Crécy (1346) ; arbalétriers et archers devant les ailes dans le **plan de bataille français de
  1415** (British Library, Cotton Caligula D. V, publié par Christopher Phillpotts, « The French
  Plan of Battle during the Agincourt Campaign », *English Historical Review* 99, 1984) ;
- **derrière**, faute de place, à Azincourt même (Monstrelet ; Curry, *Agincourt : A New History*,
  2005) : ce fut une erreur, les tireurs français n'ont presque pas tiré.

**Placement recommandé** (ordre de grandeur) : infanterie au centre, unités à 10-15 m les unes des
autres ; tireurs à pied 30-50 m **devant** l'infanterie (ils se replient par les intervalles au
contact) ; montés sur les deux ailes, au niveau de la ligne ou 15-30 m en retrait ; engins 80-120 m
derrière ; général 40-80 m derrière le centre.

**Décision de jeu** : garder les tireurs **derrière** (placement actuel, non-régression, et le jeu
fait peut-être tirer par-dessus) ou les passer **devant** (plus historique pour l'ouverture). Si le
placement actuel est gardé, écrire dans la description que les tireurs sont « en arrière de la
ligne », pas « devant ».

**description_fr** : « L'ordre ordinaire : les gens de pied au centre, les tireurs en arrière de
la ligne, les hommes d'armes montés sur les ailes, les engins derrière. »
(Variante si les tireurs passent devant : « …les tireurs en avant pour ouvrir le combat… ».)

## 2. La herse (défense à l'anglaise)

**Le mot.** Froissart, pour Crécy : les Anglais ont « leurs archers mis en manière d'une herse, et
les gens d'armes au fond » (*Chroniques*, éd. Luce, SHF, t. III) ; il reprend l'image à Poitiers.
Une herse est l'instrument agricole (cadre à dents) ; certains y voient la herse de porte ou le
chandelier triangulaire des ténèbres.

**Le débat.**
- **A. H. Burne**, *The Crecy War* (1955) et *The Agincourt War* (1956) : la herse est un **coin
  (triangle) d'archers** avancé **entre** les batailles d'hommes d'armes, pointe vers l'ennemi.
  Appui : les *Gesta Henrici Quinti* (éd. Taylor et Roskell, 1975) parlent de *cunei* d'archers
  insérés dans chaque bataille à Azincourt. Cette lecture a longtemps été classique.
- **Jim Bradbury**, *The Medieval Archer* (1985) : *cuneus* signifie simplement « un corps de
  troupe » en latin médiéval ; la herse évoque un ordre **ouvert, en quinconce**, sur plusieurs
  rangs, pas un triangle ; rien ne prouve des coins entre les batailles.
- **Anne Curry**, *The Battle of Agincourt : Sources and Interpretations* (2000) et *Agincourt :
  A New History* (2005) : à Azincourt, les archers sont **sur les ailes** (Monstrelet, Jean Le
  Fèvre de Saint-Rémy et Jean de Waurin, ces deux derniers présents), les hommes d'armes au centre
  en trois batailles jointives sur une seule ligne, peu profonde.
- **Clifford J. Rogers**, *War Cruel and Sharp* (2000), et (dir.) *The Wars of Edward III :
  Sources and Interpretations* (1999) pour Crécy : archers **sur les ailes**, avancés et
  légèrement rabattus, capables de prendre de flanc un assaillant canalisé vers le centre ;
  peut-être un mince rideau devant. Strickland et Hardy, *The Great Warbow* (2005), vont dans le
  même sens. DeVries (1996) doute lui aussi des coins.
- **Consensus actuel** : archers en masses sur les ailes, avancées ; « herse » décrit leur ordre
  ouvert et hérissé (et, après 1415, les pieux), pas des triangles entre les batailles.

Le préréglage proposé par le plan (hommes d'armes à pied au centre, archers en ailes avancées
obliques, montés en réserve derrière) **suit le consensus** : c'est bien.

**Précisions historiques pour le placement.**
- Hommes d'armes (à pied) : au centre, en deux ou trois batailles jointives (à Azincourt : York à
  droite, le roi au centre, Camoys à gauche), écarts faibles (5-10 m).
- Archers : sur les deux ailes, **avancés de 20-50 m** par rapport à la ligne des hommes d'armes,
  **orientés vers l'intérieur de 15-30°** ; option : un rideau mince d'archers devant le centre.
- Montés : **réserve** 80-150 m derrière (à Crécy, les chevaux sont au parc, derrière l'armée, et
  le roi garde une bataille en réserve au moulin ; à Poitiers, la petite réserve montée du captal
  de Buch décide la bataille en tournant les Français, selon Froissart et le héraut Chandos).
- Général : avec la bataille du centre, ou en retrait sur une hauteur (Édouard III au moulin).
- Engins : derrière, ou absents.

**description_fr** : « Défense à l'anglaise, comme à Crécy et à Azincourt : les hommes d'armes à
pied au centre, les archers en avant sur les deux ailes, tournés vers l'intérieur, et une réserve
montée en arrière. »

## 3. Trois batailles

**Historique.** L'ordre français par excellence, et en fait l'ordre commun de toute l'Europe :
avant-garde, bataille, arrière-garde. Les Français les rangent souvent **l'une derrière l'autre** :
- Crécy (1346) : les batailles françaises, arrivées en ordre de marche, attaquent l'une après
  l'autre (Jean le Bel, *Chronique*, éd. Viard et Déprez, t. II ; Froissart) ;
- Poitiers (1356) : un détachement monté des maréchaux, puis trois batailles à pied successives
  (le dauphin, le duc d'Orléans, le roi) ;
- Azincourt (1415) : avant-garde et bataille à pied, arrière-garde montée, plus des ailes montées
  chargées de disperser les archers (plan de 1415, Phillpotts, 1984 ; Curry, 2005).
Les Anglais, eux, placent souvent leurs trois batailles **côte à côte** (Azincourt) : c'est la
herse. Les Écossais rangent leurs trois ou quatre schiltrons en échelons (Halidon Hill 1333,
Neville's Cross 1346 ; Sumption, *Trial by Battle*, 1990) : ce préréglage leur convient aussi.

**Critique historique** à glisser dans le Codex, pas dans l'infobulle : les batailles successives
se font battre en détail si elles sont trop espacées (Poitiers) ou s'écrasent si elles sont trop
serrées (Azincourt).

**Placement recommandé** :
- avant-garde : la meilleure infanterie (ou les hommes d'armes démontés) avec les tireurs devant
  elle (30-50 m) ;
- bataille : 60-150 m derrière l'avant-garde, même largeur, avec le **général** ;
- arrière-garde : 60-150 m derrière la bataille ; montés et reste de l'infanterie, prêts à
  appuyer ou à couvrir la retraite ;
- ailes montées (facultatif) : une partie des montés sur les flancs de l'avant-garde, pour
  charger les tireurs ennemis (plan de 1415) ;
- engins : entre la bataille et l'arrière-garde.
Répartition des unités d'infanterie : environ un tiers par bataille, la plus forte au centre si
le nombre ne tombe pas juste.

**description_fr** : « L'ost en trois batailles l'une derrière l'autre : avant-garde, bataille du
chef, arrière-garde. Chaque ligne peut relever ou appuyer la précédente. »

## 4. Charge → « Charge de la chevalerie »

**Historique.** Après Poitiers (1356), la chevalerie française combat surtout **à pied** en
bataille rangée (Cocherel 1364, Auray 1364, Nájera 1367, Azincourt 1415, Verneuil 1424 en grande
partie) ; la charge montée reste décisive dans des cas précis :
- sur un flanc ou contre un ennemi déjà engagé : **Roosebeke, 1382** (les ailes montées françaises
  prennent la masse flamande de flanc pendant que le centre à pied la fixe) ;
- **par surprise**, contre des tireurs non retranchés : **Patay, 1429** (l'avant-garde de La Hire
  tombe sur les archers de Talbot avant qu'ils n'aient planté leurs pieux ; témoignage de Jean de
  Waurin) ;
- **en fin de bataille**, contre un ennemi fixé ou ébranlé : Formigny (1450, arrivée de Richemont
  sur le flanc), Castillon (1453, cavaliers bretons).
Contre des archers retranchés et prévenus, la charge frontale échoue (Crécy, Azincourt). Contamine,
*Guerre, État et société* (1972) ; Rogers (2000) ; Sumption, *Trial by Fire* (1999).

**Critique du plan.** « Cavalerie en première ligne, fantassins en soutien, tireurs sur les ailes »
est plausible au début de la période (1337-1356) et pour une attaque de flanc ; il faut que
l'infanterie **suive** assez près pour exploiter, et que les tireurs soient **devant les ailes**
pour préparer le choc, non en retrait.

**Placement recommandé** :
- chevalerie : première ligne, par **conrois** avec des intervalles (15-25 m) pour ne pas se gêner,
  éventuellement en deux échelons (le second 40-60 m derrière) ;
- tireurs à pied : sur les ailes, 10-30 m en avant de la chevalerie, pour ouvrir le combat puis
  s'écarter ;
- infanterie : 80-150 m derrière la chevalerie, pour exploiter la brèche ;
- général : avec le second échelon de chevalerie (le chef à cheval mène la charge, mais pas en
  tête de tout) ;
- engins : en arrière.

**Nom** : « Charge de la chevalerie » (ou « Assaut monté ») plutôt que « Charge » tout court, pour
ne pas confondre avec la charge d'une unité (CB2).

**description_fr** : « Les hommes d'armes montés en première ligne, par conrois, les tireurs sur
les ailes et les gens de pied qui suivent pour exploiter. À réserver à un ennemi surpris, fixé ou
pris de flanc : contre des archers retranchés, la charge se brise. »

## 5. Colonne → « Ordre de marche »

**Historique.** En marche, l'armée garde ses batailles dans l'ordre : **coureurs** (éclaireurs
montés) en tête, **avant-garde**, **bataille** (le chef, souvent le charroi à sa suite),
**arrière-garde** qui couvre le charroi et ramasse les traînards. C'est l'ordre de la chevauchée
de 1346 et de celle de 1356 (Rogers, 2000 ; Sumption, 1990), et c'est pour cela qu'on peut « se
mettre en bataille » vite : chaque bataille fait face sur place. Ranger les unités « par ordre de
vitesse » n'est **pas** historique et casse l'armée en morceaux : l'étirement de la colonne est
précisément ce qui perd les Anglais à Patay (1429) et ce qui permet aux Français de rattraper le
Prince noir avant Poitiers.

**Placement recommandé** :
- file unique (ou deux si le terrain le permet) le long de la direction de marche ;
- ordre : montés légers en tête (hobelars, jinetes, archers montés), puis une avant-garde mêlant
  infanterie et tireurs, puis le **général** avec la bataille, puis les engins, puis l'arrière-garde
  (le reste des montés et de l'infanterie) ;
- intervalle entre unités : 10-20 m ; **même vitesse pour tous** (`match_speed`, celle du plus
  lent) ;
- chaque unité en formation colonne (existant : 6 files, 4 à cheval).

**Posture** : ni attaque ni défense ; prévoir `stance: "march"` (ou le classer en « Défense » dans
le sélecteur, faute de mieux).

**description_fr** : « L'armée en marche, batailles dans l'ordre : éclaireurs montés en tête,
avant-garde, bataille du chef, engins, arrière-garde. Tous vont au pas du plus lent : une colonne
étirée se fait surprendre. »

## 6. Préréglages manquants

### 6.1 Bataille à pied (à ajouter)

**Historique.** L'ordre défensif dominant de 1346 à 1450, chez les Anglais puis chez les Français
et les Bretons : presque tous les hommes d'armes démontés en **une seule grande bataille** ou deux
batailles jointives, tireurs sur les ailes ou devant, **une petite réserve montée cachée** pour
tourner l'ennemi ou poursuivre. Poitiers (1356 ; la réserve du captal de Buch, Froissart et le
héraut Chandos, *Vie du Prince Noir*, éd. Tyson, 1975), Cocherel (1364, Du Guesclin, avec la
feinte de retraite et la réserve de 30 cavaliers pour enlever le captal), Auray (1364), Nájera
(1367), Verneuil (1424). La différence avec la herse : les tireurs ne sont pas nécessairement des
archers en masse, et c'est la **réserve montée de flanc** qui fait la décision.

**Placement recommandé** : infanterie (et montés destinés à démonter) au centre, compacte, écarts
5-10 m ; tireurs sur les ailes, 10-30 m en avant ; **réserve montée sur un flanc, 100-200 m en
retrait**, décalée vers l'extérieur ; général au centre.

**Note** : le préréglage place ; il ne démonte pas. Le joueur garde l'ordre de chef « Pied à
terre ». On peut signaler dans l'infobulle qu'il se combine avec cet ordre.

**description_fr** : « Tous les hommes d'armes à pied en une masse compacte, les tireurs sur les
ailes, et une petite réserve montée à l'écart pour prendre l'ennemi de flanc, comme à Poitiers et
à Cocherel. À combiner avec l'ordre « Pied à terre ». »

### 6.2 Camp retranché (à étudier)

**Historique.** Très attesté : le **parc de charrettes** fermé des Anglais à Crécy, derrière
l'armée, avec les chevaux et une seule entrée (Jean le Bel et Froissart ; Giovanni Villani, *Nuova
Cronica*, parle même de chariots autour de l'armée) ; les chariots flamands à Mons-en-Pévèle
(1304) ; le **Wagenburg** hussite (années 1420, fiche `cdx_wagenburg`) ; le camp retranché de
l'artillerie de Jean Bureau à **Castillon** (1453), fossé et palissade, où l'armée de Talbot se
brise. DeVries (1996) ; Contamine (1972).

**Problème de jeu** : sans objet « charroi » ou palissade sur le champ, un préréglage ne ferait
qu'un anneau d'unités, ce qui trompe le joueur. Recommandation : ne pas l'inclure dans CB6 ; le
garder pour un lot qui aurait des charrettes ou un retranchement de déploiement (bataille
défensive préparée). Si un jour il existe : tireurs et engins dans l'enceinte, face à l'ennemi ;
infanterie aux ouvertures ; montés dehors en réserve.

### 6.3 Écartés

- « Coins » d'archers entre les batailles (lecture de Burne) : historiographie dépassée ; ne pas en
  faire un préréglage.
- Carré ou couronne d'armée encerclée : rare et désespéré, et le schiltron couvre le cas à l'échelle
  de l'unité.
- Ordre oblique, tenaille et autres ordres « antiques » : anachroniques pour 1337-1453.

## Récapitulatif des places par rôle (ordre de grandeur)

Distances en mètres par rapport à la ligne de l'infanterie, positives vers l'ennemi.

| Préréglage | `infantry` | `foot_ranged` | `cavalry` | `siege` | `general` |
|---|---|---|---|---|---|
| Ligne de bataille (actuel) | ligne, centre | −45, centre | ailes, −15 | −95 | −40 à −80, centre |
| Ligne de bataille (variante historique) | ligne, centre | +30 à +50, centre | ailes, 0 à −30 | −80 à −120 | −40 à −80 |
| La herse | ligne, centre (2-3 blocs jointifs) | ailes, +20 à +50, rabattus de 15-30° | réserve, −80 à −150 | derrière | centre ou hauteur en retrait |
| Trois batailles | ⅓ en avant-garde (0), ⅓ à −60/−150, ⅓ à −120/−300 | devant l'avant-garde, +30 à +50 | arrière-garde, option ailes de l'avant-garde | entre bataille et arrière-garde | 2e ligne, centre |
| Charge de la chevalerie | −80 à −150, centre | ailes, +10 à +30 devant la chevalerie | 1re ligne (0 = ligne des montés), conrois espacés de 15-25 m, option 2e échelon à −40/−60 | derrière | 2e échelon |
| Ordre de marche | file : avant-garde puis arrière-garde | avec l'avant-garde | montés légers en tête, montés lourds à l'arrière-garde | après la bataille | avec la bataille |
| Bataille à pied | ligne compacte, centre (écarts 5-10 m) | ailes, +10 à +30 | réserve sur un flanc, −100 à −200, décalée vers l'extérieur | derrière | centre |

## Points qui demandent une décision de jeu

1. **Tireurs de la « Ligne de bataille »** : derrière (placement actuel, 45 m) ou devant (plus
   historique). Le plan dit « devant », le code fait « derrière » : corriger l'un ou l'autre.
2. **Ordre de marche** : ordre par batailles et vitesse commune, au lieu de « par ordre de vitesse ».
3. **« Charge » renommée** « Charge de la chevalerie », avec infanterie qui suit et tireurs devant
   les ailes.
4. **Ajouter « Bataille à pied »** (recommandé) ; le raccourci Alt+Maj+1…5 devient 1…6.
5. **Camp retranché** : hors CB6, faute d'objet charroi ; à garder pour un lot futur.
6. **Posture de la colonne** : prévoir `stance: "march"` dans le schéma, ou la ranger sous
   « Défense ».

## Sources

**Chroniqueurs** : Jean le Bel, *Chronique*, éd. Viard et Déprez, SHF, 1904-1905 ; Froissart,
*Chroniques*, éd. Luce, SHF, t. III et V ; Geoffrey le Baker, *Chronicon*, éd. Thompson, 1889 ;
héraut Chandos, *La Vie du Prince Noir*, éd. Tyson, 1975 ; Giovanni Villani, *Nuova Cronica* ;
*Gesta Henrici Quinti*, éd. Taylor et Roskell, 1975 ; Monstrelet, *Chronique*, éd. Douët-d'Arcq,
SHF ; Jean Le Fèvre de Saint-Rémy, *Chronique*, éd. Morand, SHF ; Jean de Waurin, *Recueil des
croniques*, éd. Hardy, Rolls Series. Plan de bataille français de 1415 : BL Cotton Caligula D. V.

**Historiens** : A. H. Burne, *The Crecy War*, 1955, et *The Agincourt War*, 1956 ; Jim Bradbury,
*The Medieval Archer*, 1985 ; Anne Curry, *The Battle of Agincourt : Sources and
Interpretations*, 2000, et *Agincourt : A New History*, 2005 ; Clifford J. Rogers, *War Cruel and
Sharp*, 2000, et (dir.) *The Wars of Edward III*, 1999 ; Kelly DeVries, *Infantry Warfare in the
Early Fourteenth Century*, 1996 ; Christopher Phillpotts, « The French Plan of Battle during the
Agincourt Campaign », *EHR* 99, 1984 ; Matthew Strickland et Robert Hardy, *The Great Warbow*,
2005 ; Jonathan Sumption, *Trial by Battle*, 1990, et *Trial by Fire*, 1999 ; Philippe Contamine,
*Guerre, État et société à la fin du Moyen Âge*, 1972, et *La Guerre au Moyen Âge*, 1980 ;
J. F. Verbruggen, *The Art of Warfare in Western Europe during the Middle Ages*, 2e éd., 1997 ;
Andrew Ayton et Philip Preston, *The Battle of Crécy, 1346*, 2005.
