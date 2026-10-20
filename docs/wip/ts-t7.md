# TS-T7 sous-bois et touffes (branche ts/t7)

État : FAIT, vérifié par tests (12 tests de placement) et dump. Pas de capture.

- `client/src/campaign/countryside/understory.rs` : réglages + semis pur (ceinture de lisière par grille de
  distance à la forêt, touffes pâles, touffes proches) + maillage ; `understory_layer.rs` : flux par cellules
  (threads, budget qualité, fondu par seuil alpha, teinte par saison lissée) ; réglages `data/fx/understory.json`
  (schéma `fx_understory.schema.json`).
- Dump : `clutter.understory.{far,near}` (cellules, pièces semées/affichées par catégorie, budget).
- Hors périmètre / restes : « quelques rochers » de la ceinture non faits (les affleurements restent à
  `rocks.rs`) ; pas de lande (aucun masque dédié) : la toundra = altitude sous la limite des arbres ;
  pas de capture de contrôle du rendu ; budgets à régler sur machine calme ; `treeline_m` passé en `pub(crate)`
  (vegetation/mask.rs) ; en hiver, les fleurs/touffes vertes ne deviennent pâles que par la teinte de saison
  (pas de re-semis par masque de neige au sol des touffes proches au-delà de `snow_max`).
