# TS-T7c — roche du sous-bois + objets de vie des clairières

Branche `ts/t7c`.
## État
- Tâche 1 (fait) : `RockRule` dans `understory.rs` ; données `data/fx/understory.json` `rock.{max_share,slope_gain,max_altitude_m}` ; part de roche = `steep*high` du shader terrain (bande `ground_look` de `campaign_seasons.json`, via `SnowModel.rock_band`) avec pente affichée = pente réelle × `slope_gain` (≈ HEIGHT_SCALE × 719 m/px × exagération) ; roche d'office au-dessus de `max_altitude_m`. Rejet dans belt / pale / near. Test unitaire.
- Tâche 2 (fait) : `vegetation/clearing_life.rs` ; données `clearings.life` de `data/art/campaign_forest_look.json` (objets par id de `kinds`, glb `models/<model>_lod<lod>.glb`, budget par qualité, portée, taille façon décor). Sites = `ForestClearings`.
## Reste
- Contrôle visuel par le joueur (pas de capture prise) ; échelle réelle des glb non mesurée (taille = `size_m` indicatif, facteur d'écran comme le décor).
- Valider `slope_gain`/`max_altitude_m` à l'oeil sur la vue 5.9,45.25,55.
