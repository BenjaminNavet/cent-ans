# SR — Figurines semi-réalistes (« de vrais soldats en armure »)

Date : 2026-09-30. Statut : décidé en autonomie (mandat du joueur du 30/09 : semi-réaliste
pour campagne et bataille, figurines qui ressemblent à de vrais soldats en armure, NB2 ≈ 20 $
au total, sans variantes).
Contexte technique : `docs/wip/fg-figurines-fines.md`, ADR 0088 (FG3), 0104 (GA1), bible DA § 6.

## 1. Constat (30/09)

Les figurines fines (FG) ont une bonne géométrie mais un rendu « plastique » :
- matières de détail GA1 **générées** (gpt-5-image-mini) : normale et rugosité déduites de la
  seule luminance, trame peu crédible de près ;
- métal à rugosité quasi uniforme, aspect chromé ; pas de carte de métal ;
- **aucune usure** : ni boue, ni poussière, ni crasse dans les creux, ni rayures, ni rouille
  (seuls le sang et les moignons existent) ;
- silhouettes d'équipement jugées sur les recettes FG2, jamais confrontées à des références
  réalistes.

## 2. Lots

### SR1 — Matières scannées CC0 (0 $)
Remplacer 8 des 12 couches GA1 par de vrais scans PBR **ambientCG (CC0)**, en gardant l'ordre
des couches, les formats et le chargement (`fine_detail_ga1.png`, `fine_detail_albedo.png`) :

| Couche | Scan |
|---|---|
| WOOL 0 | Fabric045 |
| LINEN 1 | Fabric061 |
| FUSTIAN 2 | Fabric066 |
| GAMBESON 3 | Fabric048 |
| MAIL 4 | Chainmail002 |
| LEATHER 5 | Leather033A |
| PLATE 6 | Metal055A |
| WOOD 7 | Wood049 |

SKIN, HAIR, COAT_LIGHT, COAT_DARK restent GA1. Chaîne : `material_gen` accepte
`source: ambientcg:<Id>` dans `materials.yaml` ; téléchargement 1K-JPG (Color, NormalGL,
Roughness, Displacement) dans `~/dev/cent-ans-raw/sr1/` ; normale **du scan** (plus de
normale déduite) ; relief = Displacement ; albédo centré comme GA1 ; recadrage à l'échelle
physique : `tile_m` choisi pour des mailles de ~9 mm, une trame de lin/laine réaliste, etc.,
et `GA1_TILE_SIZE` du shader mis à jour en conséquence (test de cohérence yaml ↔ shader ajouté).
Provenance : `SOURCE.md` + `CREDITS.md`.

### SR2 — Usure et métal crédible (0 $, shader)
Dans `battle_soldier_skinned.gdshader` (variante FG3_BAKED, < 80 m), uniforme `weathering`
(0-1, défaut 0,6) et drapeau `--no-sr2` :
- **boue et poussière** montant des pieds (hauteur de repos `v_rest.y`, jusqu'aux genoux,
  bord bruité), quantité propre à chaque figurine (`v_var`), plus forte sur étoffe et cuir ;
  les cavaliers : bas des jambes et du caparaçon ;
- **crasse des creux** : l'AO cuit assombrit et désature davantage étoffe et cuir ;
- **acier vivant** : rugosité de plate variée (0,25-0,6) par taches, liseré d'usure plus clair
  sur les arêtes (bruit × AO inversé), légère oxydation brune en bas des mailles ;
- **teintes naturelles** : saturation des livrées de près ramenée vers des teintures végétales
  (−15 %), pas d'effet sur la lisibilité lointaine (`far_boost` inchangé).

### SR3 — Références réalistes et silhouettes (≤ 1 $ NB2)
NB2, **une image par type**, 1K 3:2, style « photographie de reconstitution historique,
armure de musée 1340-1360 », face/profil/dos sur une planche : homme d'armes à pied, archer
anglais, arbalétrier génois, piquier/sergent, chevalier monté, milicien. Pas d'ancre
(registre 3D). Les planches servent à confronter les recettes FG2 : liste d'écarts de
silhouette (bassinet à camail, gantelets, cotte d'armes courte/jupon, écu, chapel de fer,
brigandine…) puis corrections des recettes Blender et recuisson (agent `cent-ans-dev`).
Pas de TRELLIS pour les soldats (animation et squelette imposent les modèles Blender).

### SR4 — Contrôle (session principale)
Captures A/B `--closeup` avant/après (2 au plus) ; perf A/B (`--benchmark --units=20`,
≤ +5 %) ; `ga1_maps_test.gd`, `fg3_maps_test.gd`, `smoke.gd`. ADR 0136.

## 3. Budget
NB2 : ≤ 1 $ (SR3). Enveloppe NB2 globale ≈ 20 $ (3,10 $ déjà en NB). Section « SR » de
`docs/budget.md`.

## 4. Hors périmètre (suite du mandat semi-réaliste)
Campagne 3D : imposteurs d'arbres lointains (avec le chantier FPS carte), bâtiments. Parchemin
peint de la vue stratégique : reste un registre carte, à reprendre après SR.
