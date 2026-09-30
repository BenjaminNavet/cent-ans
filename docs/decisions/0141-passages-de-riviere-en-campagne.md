# ADR 0141 — Passages de rivière sur la carte de campagne

Date : 2026-09-30. Statut : accepté. Chantier RC (suivi : `docs/wip/rc-rivieres-campagne.md`).

## Contexte

Retour du joueur : les fleuves se voient mal sur la carte de campagne, les rivières sont trop rares,
leurs noms n'apparaissent nulle part, et ils ne pèsent pas assez dans la stratégie. La grille de
navigation ferme déjà 16 grands fleuves hors ponts et gués (lot M1), mais au combat la seule trace
d'une rivière est un drapeau de province (`province.rivers` non vide → assaillant −20 %), sans
rapport avec l'endroit où l'on se bat. La bataille tactique (ADR 0033) sait jouer un pont en goulet,
mais ne reçoit pas le passage de la campagne.

## Décision

- **Cœur** : le passage réel décide. Le cœur charge `data/map/crossings_px.json` (ponts historiques,
  gués, bacs, ponts de route). Au déclenchement d'une bataille de campagne, si un passage se trouve
  à moins de `search_radius_km` du lieu du combat et que l'assaillant et le défenseur sont sur des
  rives opposées (côté de la ligne du fleuve donné par `dir`), la bataille est **une bataille de
  passage** : coefficient d'assaillant selon la structure (pont de pierre, de bois, de bateaux, bac,
  gué) et bonus des tireurs du défenseur. Ce coefficient remplace le drapeau de province (inchangé
  hors passage, pour ne pas dérégler l'équilibrage). Nombres dans `data/rules/river_crossings.json`
  (schéma `river_crossings_rules.schema.json`).
- **Pronostic** : la ligne de modificateur nomme le passage et le fleuve (« Passage en force du pont
  des Tourelles (Orléans) sur la Loire (−40 %, tireurs du défenseur +25 %) »).
- **Largeur du lit** : un pont de route sur un ruisseau ne vaut pas un pont sur la Loire : l'effet est
  proportionnel à la largeur du passage entre `min_width_px` et `full_width_px` (ponts et gués
  historiques : effet plein) ; sous le minimum, pas de bataille de passage.
- **IA stratégique** : elle ne lit pas le pronostic ; `attack_order` (`ai/src/grid.rs`) multiplie sa
  puissance par le coefficient du passage qui la sépare de sa cible : elle ne force un pont tenu que
  nettement plus forte.
- **Bataille tactique** : `BattleSetup.crossing` (structure, nom, fleuve) ; le champ reçoit une
  rivière entre les deux lignes avec ce seul passage (goulet), l'IA de berge d'ADR 0033 le tient.
  Graines et batailles sans passage inchangées.
- **Rendu** (Godot) : fleuves plus larges et plus contrastés, rivières mineures visibles plus loin,
  noms des cours d'eau en petit italique le long du lit (`Label3D`, noms français de
  `data/map/river_names.json`), selon l'importance et le zoom.
- **Densité** : les rivières moyennes du réseau fin (BD TOPAGE, OS Open Rivers, EU-Hydro, lot ZG5a)
  peuvent être versées dans la couche de campagne (`cent-ans geo rivers-render --fine-min-order N`)
  sur une machine qui a la pyramide ; rendu seulement, la grille de navigation n'est pas refaite.
- **Matières d'eau** : textures de détail tuilables (mer, océan, fleuve, rivière) générées par
  Nano Banana 2 (`google/gemini-3.1-flash-image` via OpenRouter, chaîne GA de `material_gen.py`),
  plafond 5 $ ; les shaders gardent le rendu procédural quand la texture manque.

## Conséquences

- Les ponts deviennent des verrous : tenir la tête de pont vaut une armée plus forte ; contourner
  par un gué éloigné ou un autre pont devient un vrai choix.
- Nouveaux réglages à équilibrer (sonde `balance_probe`).
- Onze rivières de plus deviennent infranchissables hors passages (Marne, Yonne, Vienne, Charente,
  Lot, Tarn, Allier, Cher, Moselle, Severn, Trent ; 40 ponts, gués et bacs médiévaux ajoutés à
  `crossings.json`, grille régénérée) ; les rivières ajoutées pour la densité (réseau fin) ne
  bloquent pas.
