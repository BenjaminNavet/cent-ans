# Histoire et savoir — conception (session « historien »)

Date : 2026-09-23. Auteur : session historien (Claude), à la demande de Benjamin.
Suivi : `docs/wip/historien.md`.

## 0. Intention

Le jeu doit **faire apprendre** : chaque nom, lieu, plat ou plante croisé en jouant ouvre une
bulle d'information, et de bulle en bulle le joueur explore l'époque (à la manière des mots-clés
de *Baldur's Gate 3*). La véracité compte mais cède au jeu : un anachronisme léger est accepté
s'il est **signalé** dans la fiche (champ `anachronism`).

Quatre chantiers :

| Lot | Contenu | Où vivent les règles |
|---|---|---|
| H1 Audit historique | Vérifier et corriger personnages, factions, événements, techs, bâtiments, unités, toponymes | `data/` + rapport `docs/histoire/audit-2026-09-23.md` |
| H2 Codex et bulles | Fiches `data/codex/`, mots « découverte », bulles imbriquées, fenêtre Codex, suivi des découvertes | `data/codex/`, `game/` (présentation pure) |
| H3 La Table (cuisine) | Régime alimentaire par province, Carême, recettes | `core/` (règles) + `data/diets/` |
| H4 Médecine et herboristerie | 3e arbre de technologies, bâtiments, herbier | `core/` + `data/technologies/`, `data/buildings/` |

## 1. H2 — Codex et bulles imbriquées

### 1.1 Données : `data/codex/<id>.json` (schéma `data/schemas/codex.schema.json`)

```json
{
  "id": "cdx_charles_v",
  "title": "Charles V le Sage",
  "category": "personnage",
  "aliases": ["Charles V", "Charles le Sage", "dauphin Charles"],
  "era": { "from": "1338", "to": "1380" },
  "summary": "Roi de France (1364-1380). Restaure le royaume après [[cdx_poitiers|Poitiers]] avec [[cdx_du_guesclin|Du Guesclin]].",
  "body": "Texte long (3-8 paragraphes) avec liens [[cdx_x]] ou [[cdx_x|libellé]].",
  "anachronism": "Optionnel : liberté prise par le jeu.",
  "entity": "chr_charles_v",
  "see_also": ["cdx_librairie_du_louvre"],
  "sources": ["Charles V (roi de France)"]
}
```

- `category` ∈ `personnage, dynastie, lieu, bataille, evenement, institution, societe, guerre,
  religion, economie, cuisine, ingredient, recette, medecine, plante, savoir, vie_quotidienne`.
- `summary` : 1 à 3 phrases, affiché dans la bulle. `body` : fiche complète dans la fenêtre Codex.
- `entity` : lien optionnel vers une entité de jeu (`chr_`, `evt_`, `tech_`, `bld_`, `unit_`,
  `prov_`, `fac_`, `diet_`, `res_`) ; la bulle de l'entité et la fiche se renvoient l'une à l'autre.
- Liens : `[[cdx_id]]` (libellé = `title`) ou `[[cdx_id|libellé]]`. Tout texte de `data/`
  (descriptions, textes d'événements) peut en contenir.
- Validation (`tools`, pytest) : schéma, liens résolus, alias uniques, pas de fiche orpheline
  sans lien entrant hors catégories racines.

### 1.2 Mots « découverte »

- `CodexText.format(text)` (GDScript) convertit les `[[…]]` en `[url=cdx:id]` stylés (encre
  rouge soulignée, comme une rubrique de manuscrit) et **auto-lie les alias** dans les textes
  produits par la simulation (chronique, messages) : première occurrence seulement, mots entiers.
- Un mot déjà découvert passe en encre brune ; un mot jamais lu reste rubriqué (rouge) : on
  voit d'un coup d'œil ce qui reste à explorer.

### 1.3 Bulles imbriquées (style BG3)

Les infobulles natives de Godot ne sont pas interactives ; on ajoute une couche dédiée :

- Autoload `CodexBubbles` (CanvasLayer) gérant une **pile de bulles** (`PanelContainer` +
  `RichTextLabel`, thème parchemin, largeur 340).
- Survol d'un mot-lien ≥ 0,35 s → bulle fille ancrée près du mot. La souris peut entrer dans
  la bulle ; survoler un lien de la bulle ouvre la suivante (pile jusqu'à 6 ; au-delà, la plus
  ancienne se ferme). Quitter toutes les bulles → fermeture après 0,4 s de grâce.
- Clic sur un lien → ouvre la fiche complète dans la fenêtre Codex. Clic droit → épingle la bulle.
- **Épingler une infobulle native** : touche `T` pendant qu'une infobulle riche (F2) est
  affichée → elle devient une bulle interactive au même endroit, dont les mots sont cliquables.
- Chaque bulle ouverte marque la fiche comme **découverte**.

### 1.4 Fenêtre Codex

- Accès : bouton du bandeau supérieur + touche `K`. Onglets par grande famille (Personnages et
  dynasties / Lieux / Guerre et batailles / Société et institutions / Religion / Table / Médecine
  et herbier / Savoirs), liste à gauche, fiche à droite, barre de recherche, compteur
  « 87 / 240 découvertes ».
- Fiches non découvertes : titre visible en grisé, contenu « À découvrir… » (le joueur peut
  quand même l'ouvrir : l'ouvrir la découvre ; pas de verrou punitif).
- Historique précédent/suivant comme un navigateur.
- Les découvertes sont une **méta-progression du joueur** (fichier `user://codex.json`),
  pas un état de partie : ce n'est pas une règle de jeu, elle reste côté Godot.

### 1.5 Coordination avec F8 (encyclopédie)

Le Codex **est** la base de l'encyclopédie F8 : F8 y ajoutera des fiches générées depuis les
données (unités, bâtiments, techs) sous la même fenêtre. Prévenir l'orchestrateur.

## 2. H3 — La Table (régime alimentaire)

### 2.1 Règle

Chaque province a un **régime** (ordre `SetDiet { province, diet }`, défaut `diet_bread_pottage`).
Un régime a : conditions (ressources accessibles à la faction, province côtière ou port,
technologie, bâtiment), coût par saison en livres (proportionnel à la population, en
milliers), effets (liste d'`Effect` existants, éventuellement ciblés par classe) et des
**modificateurs saisonniers**. Changer de régime : effet dès la saison suivante, pas de coût
de changement, un changement par province et par tour.

**Carême** (printemps) et jours maigres : un régime `lent_rule: "meat"` subit au printemps
−piété et +mécontentement du clergé ; un régime `lent_rule: "fish"` gagne de la piété.
**Hiver** : les régimes frais (`winter_rule: "fresh"`) coûtent +50 %.

### 2.2 Régimes (données `data/diets/`)

| id | Nom | Conditions | Effets principaux | Codex |
|---|---|---|---|---|
| `diet_bread_pottage` | Pain bis et potage | — | neutre | pain, potage, four banal |
| `diet_pulses` | Fèves, pois et lentilles | `tech_three_field_rotation` | +santé paysans, +croissance | assolement, légumineuses |
| `diet_lenten_fish` | Hareng saur et poisson de carême | `res_fish` + `res_salt` accessibles | +piété, bonus de Carême, coût modéré | hareng, Hanse, jours maigres, [[bataille des Harengs]] |
| `diet_meat_salting` | Lard, bœuf et salaisons | `res_salt` | +santé, −mécontentement paysans et soldats (+moral des armées levées), coût élevé, malus de Carême | salaison, gabelle (1341) |
| `diet_dairy` | Laitages, fromages et œufs | province non urbaine (terrain plaine/colline) | +santé, coût faible, malus de Carême (œufs et laitages interdits) | beurre de Carême, « tour de beurre » de Rouen |
| `diet_wine_bread` | Pain blanc et vin | `res_wine` | +bourgeois satisfaits, +richesse, coût moyen | vin de Gascogne, clairet, commerce de Bordeaux |
| `diet_spiced_table` | Table épicée à la mode de Taillevent | `bld_market` ou `bld_fair` | +prestige, +loyauté noblesse, coût très élevé | Viandier, cameline, poivre, maniguette, hypocras, blanc-manger |

Les valeurs exactes sont posées par l'agent d'implémentation, équilibrées pour qu'aucun régime
ne domine : chaque bonus a un coût ou une condition.

### 2.3 Présentation

Section « La Table » du panneau de province : régime actuel (icône), liste déroulante des
régimes disponibles avec infobulle (effets, coût, conditions manquantes, mots codex).
Au printemps, un bandeau « Carême » rappelle la règle.

## 3. H4 — Médecine et herboristerie

### 3.1 Règle

Troisième branche `TechBranch::Medicine` (onglet « Médecine » du panneau des techs), avec
nouvel effet `ResearchMedicine` ; la recherche médicale est alimentée par les monastères
(abbaye, jardin des simples), les universités et l'hôtel-Dieu. Nouveaux effets :
`PlagueResistance` (réduit la probabilité et la gravité des pestes et épidémies locales) et
`WoundRecovery` (part des pertes de bataille récupérées comme blessés après la bataille).
Chaque technologie a une liste `herbs` d'ids codex : les plantes entrent dans l'**Herbier**
(catégorie `plante`) et y sont marquées découvertes quand la tech est acquise.

### 3.2 Arbre (données)

| Tier | Tech | Année / repère | Effets indicatifs | Plantes |
|---|---|---|---|---|
| 1 | `tech_herb_garden` Jardin des simples | capitulaire *De Villis* (v. 800), plan de Saint-Gall | débloque `bld_herb_garden`, +santé | sauge, rue, menthe, fenouil |
| 1 | `tech_humoral_theory` Théorie des humeurs | Hippocrate, Galien, Avicenne (*Canon*) | +recherche médicale | — |
| 2 | `tech_regimen_sanitatis` Régime de santé de Salerne | *Regimen sanitatis Salernitanum* (XIIe-XIIIe) | les régimes alimentaires donnent +25 % de santé | ail, oignon, hysope |
| 2 | `tech_willow_bark` Remèdes contre les fièvres | écorce de saule, reine-des-prés | +santé, −mortalité en hiver | saule, reine-des-prés, camomille |
| 2 | `tech_barber_surgeons` Barbiers-chirurgiens | corporation des barbiers | +récupération des blessés, +résistance à l'attrition | plantain, consoude, millepertuis |
| 3 | `tech_theriac` Thériaque et apothicaires | thériaque de Venise ; apothicaires-épiciers | débloque `bld_apothecary`, +santé, +richesse bourgeois | thériaque, aloès, safran |
| 3 | `tech_montpellier` Faculté de médecine de Montpellier | statuts de 1220, bulle de 1289 | +recherche médicale, prérequis | — |
| 3 | `tech_soporific_sponge` Éponge soporifique | *spongia somnifera* (Hugues de Lucques) | +récupération des blessés, +moral | pavot, mandragore, jusquiame |
| 4 | `tech_plague_consilia` Conseils contre la peste | *Compendium de epidemia*, Faculté de Paris (1348) | +résistance à la peste | genièvre (fumigations), vinaigre |
| 4 | `tech_chauliac_surgery` Grande Chirurgie | Guy de Chauliac, *Chirurgia Magna* (1363) | +récupération des blessés, +santé | — |
| 4 | `tech_leprosaria` Léproseries et maladreries | réseau de maladreries | +résistance aux épidémies, −mécontentement | — |
| 5 | `tech_aqua_vitae` Eau-de-vie des médecins | Arnaud de Villeneuve, Jean de Roquetaillade | +santé, +commerce | romarin (« eau de la reine de Hongrie », anachronisme léger signalé) |

`tech_quarantine` (civile) et `tech_hospital_reform` (civile) sont rattachées à cet arbre si la
migration est simple ; sinon, prérequis croisés.

### 3.3 Bâtiments

- `bld_herb_garden` Jardin des simples (sanitaire, tier 1) : +santé, +recherche médicale.
- `bld_apothecary` Apothicairerie (sanitaire, tier 2, requiert `tech_theriac`) : +santé,
  +richesse bourgeois, +résistance à la peste.

## 4. H1 — Audit historique

Relecture de toutes les données historiques. Chaque correction est appliquée dans `data/` et
consignée dans `docs/histoire/audit-2026-09-23.md` (entité, avant, après, justification,
source). Classement : **erreur** (corrigée), **approximation assumée** (laissée, signalée),
**anachronisme de jeu** (laissé, documenté). Vérifications en ligne (WebSearch) pour tout
point douteux ; ne rien inventer.

## 5. Autres thèmes d'époque (demande de Benjamin : « en trouver d'autres »)

Critère : un thème n'entre dans le jeu comme **mécanique** que s'il crée un vrai choix ; sinon
il vit dans le Codex (fiches + liens), ce qui ne coûte que de la rédaction.

### 5.1 H5 — Monnaie et mutations monétaires (mécanique)

Les rois du XIVe siècle financent la guerre en « muant » la monnaie : Philippe VI et Jean II
changent la teneur en argent des pièces des dizaines de fois (plus de 80 mutations entre 1337
et 1360), avant le **franc** de 1360 (rançon de Jean II) et la monnaie forte de Charles V,
défendue par Nicole Oresme (*Traité des monnaies*, v. 1355).

- Ordre de faction `SetCoinage { level }` : `strong` (monnaie forte), `sound` (défaut),
  `debased` (affaiblie), `heavily_debased`.
- Affaiblir : revenu immédiat de monnayage (seigneuriage) chaque saison, mais inflation
  cumulative (`price_level` de la faction) qui renchérit recrutement et entretien, fait monter
  le mécontentement des bourgeois et du clergé (rentes fixes) et baisse la richesse.
- Revenir à la monnaie forte : coûteux (refonte), fait baisser lentement `price_level`,
  +loyauté des bourgeois, +prestige.
- Codex : livre tournois, écu, franc à cheval, gros tournois, Oresme, changeurs, états généraux
  de 1355-1358, Étienne Marcel.

### 5.2 H6 — Chevalerie, rançons et ordres (mécanique)

- **Rançons** : les personnages capturés en bataille (statut captif existant) ont une rançon
  fixée par rang et prestige. Le camp captif peut payer (en une fois ou par termes annuels),
  l'autre camp peut exiger une province à la place, libérer sur parole (prestige) ou garder.
  Repères : Jean II (Poitiers, 1356, trois millions d'écus au traité de Brétigny-Calais),
  Du Guesclin, Charles d'Orléans (25 ans captif en Angleterre).
- **Ordres de chevalerie** : action de faction « Fonder un ordre » (coût, prestige requis) :
  Jarretière (1348), Étoile (1351), Toison d'or (1430, Bourgogne) ; les généraux membres gagnent
  loyauté et moral, le souverain du prestige. Un seul ordre par faction.
- Codex : adoubement, tournois et pas d'armes, hérauts et héraldique (blasons déjà dans le jeu),
  Froissart, Geoffroi de Charny (*Livre de chevalerie*), les Bourgeois de Calais.

### 5.3 Thèmes Codex seulement (pas de nouvelle règle)

Rattachés à des éléments existants par des liens :
- **Héraldique** : chaque blason du jeu (factions) renvoie à une fiche (émaux, meubles,
  brisures, lecture du blason).
- **Calendrier et temps** : saisons du jeu ↔ travaux des mois (*Très Riches Heures*), fêtes,
  heures canoniales, calendrier julien.
- **Guerre et armement** : arc long, arbalète, harnois, bombardes (liés aux unités et techs).
- **Métiers et villes** : corporations, foires de Champagne, draperie flamande, Hanse, Lombards.
- **Savoirs** : universités, scriptorium et enluminure, librairie de Charles V, papier, Christine
  de Pizan, Ars nova et Guillaume de Machaut, horloges mécaniques.
- **Religion** : papauté d'Avignon, Grand Schisme, ordres mendiants, pèlerinages, reliques.
- **Vie quotidienne** : vêtements et lois somptuaires, hygiène et étuves, jeux (échecs, dés,
  jeu de paume), fabliaux.

## 6. Vagues

1. **Vague 1** (3 agents) : H1 audit (données) ; H2 infrastructure codex + ~20 fiches
   d'amorce ; H3+H4 règles Rust + données (régimes, techs, bâtiments, effets, ordre, pont).
2. **Vague 2** (3 agents) : UI Table + onglet Médecine ; H5+H6 règles Rust + données ;
   rédaction massive du codex (≈ 200 fiches) et passage de liens `[[…]]` dans les
   descriptions et textes d'événements existants.
3. **Vague 3** : UI monnaie, rançons et ordres ; événements éducatifs (cuisine, médecine,
   monnaie, chevalerie, savoirs) ; relecture historique finale ; captures ; doc joueur.
