# ADR 0121 — Relief fin dans le cadre monde, palier 1 étendu à tout le monde OM

Date : 2026-09-29. Statut : accepté. Lot OMR R7 (`docs/wip/omr-r7.md`), suite des ADR 0036, 0077
et 0115.

## Contexte

L'ADR 0115 a gardé la pyramide de relief fin (E1-E7, ≈ 2,7 Go hors git) dans l'ancien carré de
16 × 16 tuiles racines de l'Ouest, placé dans le monde 28 × 24 par `root_origin_tiles` [0, 5] :
ni recuisson ni nouveau paquet. L'Est et le Sud n'avaient que le relief de base (ETOPO 15″, 360 m),
et le relief rapproché s'arrêtait net au bord de l'ancien carré (Constantinople, Moscou, Le Caire,
Tunis, et même la Scandinavie du nord ou l'Andalousie).

Ce carré ne peut pas recevoir l'Est et le Sud : leurs tuiles tomberaient au-delà de ses 16 tuiles
racines, ou à des lignes négatives (le nord du monde est au-dessus de l'ancienne origine).

## Décision

- **Cadre monde** : la pyramide est désormais dans le cadre du monde (`root_origin_tiles` [0, 0],
  28 × 24 tuiles racines, `map.json`). Les tuiles existantes ne sont pas recuites : elles sont
  **renommées** (tuile (k, col, ligne) → (k, col, ligne + 5·2^k)), et les tuiles fines des fleuves
  et des routes (CAFV) sont réécrites avec leur adresse et leurs sommets décalés (+20 lignes E2,
  +1280 unités en y). Les manifestes versionnés (`relief_pyramid.json`, `rivers_fine.json`,
  `fine_anchors.json`) sont décalés une fois (`cent_ans_tools.geo.world_frame`).
- **Cadre du cache enregistré** : `pyramid/frame.json` (`root_origin_tiles`). Un cache sans ce
  fichier est d'avant R7, donc dans l'ancien cadre [0, 5] : `geo relief-all` le **recadre sur
  place** (renommages, quelques secondes) au lieu de le recuire ; `geo relief-reframe` fait de
  même à la main (ou depuis un autre cache par liens durs, sans place disque).
- **Palier 1 (E1-E2) sur toutes les terres du monde** depuis Copernicus GLO-90 (même licence et
  mêmes mentions que l'Ouest). Le palier est **recuit en entier** dans le cadre monde (version de
  cuisson `tier1` 3) : les tuiles de l'Ouest au bord de l'ancienne emprise GLO-90 (lon −11 → 12,
  lat 41 → 60) étaient complétées par E0 au-delà, et la base floue du rehaussement (σ 5 km) est
  maintenant calculée sur tout le monde (mêmes entrées qu'E0 : GLO-90 de l'Ouest + ETOPO). Les
  paliers 2 et 3 (E3-E7, cœur de l'Ouest et zones de détail) gardent leurs versions.
- **Pas de GLO-30 à l'Est** : E3-E4 restent limités au cœur de l'Ouest (France, Angleterre,
  Bénélux, rive gauche du Rhin). L'Est a le relief à 90 m (caméra jusqu'à ≈ 5 unités), l'Ouest
  jusqu'à 22 m sur le cœur et 2,8 m dans les zones de détail.
- **Cuisson en flux** : les tuiles GLO-90 d'un bloc de 4 × 4 tuiles E1 sont téléchargées, cuites
  puis supprimées dès qu'aucun bloc suivant ne les lit (sauf celles de l'emprise de l'Ouest,
  gardées pour `geo relief-shade`) ; arrêt propre et reprise sous 25 Gio libres.
- **Outillage** : le cadre n'est plus supposé carré (grilles d'étage, E0, masques, `FineRelief`,
  unités monde des CAFV, `detail_dem`, `landmarks_v2`) ; sa taille se déduit de ses bornes
  (`pyramid.frame_tiles`). `geo anchors-fine` et `geo towns`, bloqués depuis l'ADR 0115 tant que
  le cadre n'était pas le monde, sont de nouveau utilisables.
- Le moteur (Godot, `relief-lod`) lisait déjà `root_origin_tiles` et une grille rectangulaire :
  aucun changement.

## Conséquences

- Nouveau paquet « Cent Ans relief » (ADR 0077) : l'empreinte de cuisson change (tier1 3,
  fleuves et routes régénérés), `relief-pack` incrémente la version. Publication réservée au
  joueur, comme avant.
- Un cache d'avant R7 reste utilisable après un recadrage automatique, mais son palier 1 est
  périmé (version 2) : `relief-all` le recuit (téléchargement GLO-90 de l'Est compris), puis
  relance fleuves et routes fins.
- Cache ≈ +2 Go (E1-E2 de l'Est et du Sud) ; brut GLO-90 supplémentaire ≈ 6 Go téléchargés,
  non conservés.
- Le relief de base E0 (versionné) n'est pas modifié : à l'Est il reste ETOPO, E1-E2 y apportent
  le détail Copernicus.
