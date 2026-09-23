# Audit visuel et interface « à la Total War », version historique

Date : 2026-09-23 (session 4). État audité : `main` à `bc99774` + contenu F7 non commité de la
session d'orchestration (nouvelles factions). Les lots F1-F3 (worktrees) et la refonte visuelle
(branche `visual`) ne sont pas encore fusionnés : certains défauts ci-dessous y sont peut-être déjà
traités.

## 1. Méthode

18 captures en 1440×900 via les options `--screenshot` / `--stage=` existantes
(menu, carte : armée, province, ville, faction, cour, compétences, technologies, diplomatie, carte
diplomatique, chronique, objectifs, aide, siège, bataille en vue ; bataille rangée ; assaut).
Journaux Godot : **aucune erreur ni avertissement**. Captures clés dans `docs/img/audit-2026-09-23/`.

## 2. Défauts constatés

Gravité : **B** bogue (faux ou trompeur), **L** lisibilité, **F** finition. « Lot » = propriétaire.

| # | G | Écran | Défaut | Lot |
|---|---|---|---|---|
| 1 | B | Journal | Les événements des **autres factions** s'affichent sans nom de faction : « Le trésor est vide : les troupes grondent » apparaît alors que le joueur a 139 838 ₶ (`economy.rs:348`, message d'une autre faction). Idem batailles étrangères mêlées aux nôtres. | F3 (filtre) + F1 (texte avec nom) |
| 2 | B | Barre du haut | « Revenu : +24 816 » est le revenu **brut** ; le net réel est ≈ +4 000 (entretien 18 509). Le joueur croit gagner 6× plus qu'en réalité. Afficher le net, brut/charges en infobulle. | F2 (`map_ui.gd:148`) |
| 3 | B | Journal | Même bataille listée deux fois avec pertes identiques (« Florence attaque Vérone… 74 contre 6 » puis « Venise attaque Vérone… 74 contre 6 ») : une ligne par attaquant allié. | F1 (`movement.rs:501`) |
| 4 | B | Journal (siège, tour 1) | Journal resté sur « Printemps 1337 — Rien à signaler » en été 1337. | F3 |
| 5 | L | Tous panneaux | Fond à 96 % d'opacité (`parchment_theme.tres:8`) : les étiquettes 3D de la carte (blanches, contour noir) transparaissent à travers le panneau de province, la cour, la diplomatie, le journal. Passer à 100 %. | F2/F3 (thème) |
| 6 | L | Carte | Étiquettes de provinces énormes et superposées (Galles/Devon et Somerset, Gloucester/Tamise, Utrecht/Gueldre/Hollande, Luxembourg/Trèves), coupées par la barre du haut. | F6 |
| 7 | L | Carte | Marqueurs d'armée minuscules (une « table » bleue) avec un chiffre blanc flottant (« 3 », « 8 ») qui chevauche les noms. Impossible de savoir à qui est une armée sans cliquer. | `visual` / F6 |
| 8 | F | Carte | Frontières en escalier (visible en Champagne, Bourgogne). Rivières trop épaisses et saturées. | `visual` / F6 |
| 9 | L | Carte diplomatique | Les couleurs (guerre, alliés, vassaux) se distinguent à peine du relief : l'Angleterre en guerre n'apparaît pas rouge. | F6 |
| 10 | F | Province | Ligne de débogage « Identifiant : prov_ile_de_france (index 62) » visible du joueur (`province_panel.gd:112`). | F2 |
| 11 | F | Province | Largeur du panneau qui change entre les onglets Garnison et Ville ; motifs de refus en rouge coupés au bord ; mini-jauges de population (police 8 px) illisibles. | F2 |
| 12 | F | Barre du haut | Nom de la recherche tronqué (« Entraînement à l'arc lo ») ; bouton Chronique grisé sans raison visible. | F2/F3 |
| 13 | L | Plusieurs | Panneaux qui s'empilent (diplomatie par-dessus province, cour + province) sans règle de fermeture. | F3 |
| 14 | F | Chronique | « La bataille de L'Écluse » → « de l'Écluse » (article en minuscule dans le texte courant). | F7 (`data/events`) |
| 15 | F | Bataille | « Bataille de Île-de-France » → « Bataille d'Île-de-France » (élision) (`battle_scene.gd:168`, `movement.rs:501`). | F5 + F1 |
| 16 | L | Bataille | Caméra lointaine, régiments de quelques pixels, drapeaux et effectifs illisibles ; champ plat vide, bande grise à l'horizon. | F5 / `visual` |
| 17 | L | Bataille | Cartes d'unités tronquées (« Hommes d'armes à p », « Sergents r ») ; barre d'aide permanente en bas, trop petite pour être lue. | F2 (`battle_hud.gd`) |
| 18 | F | Assaut | Tours aux toits coniques bleus surdimensionnés, maisons posées en l'air, chevaliers « Formation : ligne » en escalade. | `visual` / F5 |
| 19 | F | Cour | Tous les « portraits » sont l'écu de la faction (attendu : portraits bloqués jusqu'au 1er octobre). | — |

Vérifications historiques faites sans défaut : âges de la cour de France en 1337 (Philippe VI 44,
Jean 18, Bonne de Luxembourg 22, Charles d'Alençon 40, Charles de Blois 18), alliances françaises
(Bohême, Castille, Naples, Écosse), vassaux (Bourgogne, Bretagne, Flandre), prétentions anglaises
(Guyenne confisquée le 24 mai 1337, Ponthieu), guerre Venise-Florence contre les Scaligeri.

## 3. Ce que Total War: Warhammer III fait bien — et sa traduction en 1337

Principe : on reprend la **grammaire** de l'interface (où se trouve l'information, combien de clics),
pas l'esthétique fantastique. Le registre visuel reste celui du **manuscrit enluminé** du XIVᵉ siècle :
parchemin, encre sépia, rubriques rouges, filets d'or sobres, sceaux de cire, héraldique exacte.
Pas de lueurs, de runes ni d'icônes flottantes géantes : tout ce qui flotte au-dessus du monde doit
pouvoir exister dans le monde (bannière, pennon, fumée de feu de camp).

### 3.1 Campagne

| TW:WH3 | Chez nous | Ancrage historique |
|---|---|---|
| Barre d'armée en bas au centre : cartes d'unités avec effectif, `7/20` emplacements | **Bandeau d'ost** : cartes d'unités (icône F2 + nom + barre d'effectif + moral), compteur `8/20 lances` ; clic = détail, glisser = séparer l'armée. Remplace la liste texte du panneau d'armée. | Les « retenues » par contrat d'endenture : capacité liée au rang du chef (Cdt). |
| Médaillon du général en bas à gauche, rang, points de compétence | **Sceau du chef** : sceau de cire à ses armes (portrait quand disponible), rang, points de compétence, ravitaillement et posture en icônes autour. | Sceaux équestres des grands seigneurs (Philippe VI, Édouard III). |
| Cercle « fin de tour » en bas à droite avec notifications empilées | **Cloche de fin de saison** entourée des alertes F3 (armée ennemie, siège, construction, dette, décision de chronique) ; clic sur une alerte = caméra. | Rythme saisonnier du jeu ; heures canoniales. |
| Minicarte en haut à droite + messages d'événements illustrés | **Minicarte-portulan** (F6) et pile de **lettres scellées** (nouvelles : faction rencontrée, déclaration de guerre, naissance, mort) avec écu. | Correspondance diplomatique, chevaucheurs du roi. |
| Trésor en haut au centre avec revenu net entre parenthèses | Trésor `139 838 ₶ (+4 218)` **net**, infobulle brut/charges ; date en toutes lettres. | Livre tournois (₶), déjà en place. |
| Étiquette de ville : nom + écu + niveau, drapeaux plantés aux abords | **Cartouche de ville** : écu du seigneur, nom, niveau d'enceinte (0-3 créneaux), icône de siège. Bannières d'armée à hampe, à l'échelle, avec effectif sur un petit phylactère. | Bannières carrées des bannerets, pennons des bacheliers. |
| Zone de déplacement surlignée d'un ruban doré, flèche de trajet | Ruban de portée (déjà en orange) plus fin, couleur or pâle, trajet en pointillés par étape. | — |
| Rapports d'événements plein écran illustrés | Chronique avec **lettrine** et vignette (déjà bien partie) ; texte rubriqué. | Grandes Chroniques de France, Froissart. |

### 3.2 Bataille

| TW:WH3 | Chez nous | Ancrage historique |
|---|---|---|
| Bandeau bas de cartes d'unités compactes (icône, effectif, barres fines) groupées | Cartes **deux fois plus étroites** : icône de classe, effectif, barres moral/fatigue/munitions fines ; regroupées par **bataille** (avant-garde, bataille du roi, arrière-garde) ; `Ctrl+1..9` groupes. | Les trois « batailles » de l'ost (Crécy, Poitiers). |
| Grandes bannières d'unités au-dessus des régiments (icône + état) | **Bannière réelle** du régiment (armes du capitaine, oriflamme, croix de Saint-Georges) au-dessus de la troupe, légèrement surdimensionnée ; icône de classe et chiffre sur un phylactère sous la hampe ; bordure qui vire au rouge quand le moral cède. | Héraldique de bataille ; lisible sans UI flottante. |
| Barre de rapport de force en haut, chrono, météo | Déjà en place ; y ajouter l'**objectif** (tenir le gué, prendre la porte). | — |
| Portrait du général + capacités en bas à gauche | **Ordres du chef** (pas de sorts) : « Montjoie Saint-Denis ! » (cri : moral +), « Déployer l'oriflamme » (pas de quartier : moral + et prisonniers perdus), « Pied à terre », « Planter les pieux », « Dresser les pavois ». Recharges longues, liées aux traits. | Oriflamme déployée à Crécy et Poitiers ; pieux anglais ; pavois génois. |
| Minicarte de bataille | Minicarte (F5) ; supprimer la ligne d'aide permanente au profit de la touche F1. | — |
| Vitesse du temps en bas à droite | Déjà présente, à regrouper en boutons icônes (pause, ×1, ×2, ×3). | — |

### 3.3 À ne pas prendre

Gros ornements dorés et crânes, aura/magie, unités monstrueuses, couleurs saturées de l'interface,
icônes 2D flottantes au-dessus de la carte, portraits animés 3D (hors budget).

## 4. Plan proposé

À faire **après** la fusion de F2 (qui possède `map_ui.gd`, `army_panel.gd`, `battle_hud.gd`) et de
F3, pour ne pas écraser leur travail ; les marqueurs et bannières 3D attendent la branche `visual`.

| Lot | Contenu | Fichiers |
|---|---|---|
| **A — corrections** (petit, tout de suite après fusion) | Défauts 1-5, 10-15. | `economy.rs`, `movement.rs`, `map_ui.gd`, `province_panel.gd`, thème, chronique |
| **F10a HUD de campagne** | Bandeau d'ost, sceau du chef, cloche de fin de saison + alertes, lettres scellées, trésor net, fermeture exclusive des panneaux. | `game/scripts/ui/army_strip*`, `general_seal*`, `end_turn_cluster*`, `map_ui.gd` (accroches) |
| **F10b HUD de bataille** | Cartes compactes par « bataille », ordres du chef (règles dans `sim-battle`), minicarte, boutons de vitesse. | `battle_hud.gd`, `core/crates/sim-battle` (ordres) |
| **F10c Bannières** | Bannières héraldiques à hampe (carte et bataille) générées par `heraldry.py` ; phylactère d'effectif. | avec la session `visual` |

Données : les ordres du chef et les capacités de retenue vont dans `data/` (schémas), aucun chiffre en dur.
