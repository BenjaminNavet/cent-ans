# FK5a — incidents sur la carte (rendu + UI)

Branche `feat/fk5-incidents` (base `integration/fk` 299233c8c). Spec
`docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.1.2, 3.4, 6, 7 ; ADR 0122.

## État
- [x] Cœur : `CampaignState::debug_offer_decision` (mise en scène, `chronicle.rs`) + pont
  `CampaignSim.debug_offer_decision(event, province) -> id` ; test `sim-campaign/tests/fk5_incidents.rs`.
- [x] `life_folk/incident_markers.gd` (`IncidentMarkers`) : sceau de cire (`HudStyle.draw_wax_seal`,
  pictogramme `hud_chronicle_decision`), pastille des tours restants, bulle IB `hud_incident`,
  écran (CanvasLayer) donc visible à tous les zooms ; clic → `ChronicleController.open_decision`,
  caméra glissée sur la province.
- [x] `CampaignLife` : `incidents` créé sauf `--no-life`, `--no-folk`, `--folk-off=incidents` ;
  rafraîchi à chaque `refresh` (avant le filtre « une fois par tour »).
- [x] `ChronicleController` : `modal_pending()` (sans les incidents quand leurs sceaux sont
  affichés ; sinon repli sur la fenêtre), `open_decision(id)`, `notify_expired(events)` (toast
  « Délai écoulé, le conseil a tranché — … » depuis l'entrée de chronique « (délai écoulé) »).
- [x] Alertes : un incident `map` n'est plus bloquant pour le bouton de fin de tour.
- [ ] Test headless `game/tests/fk5_incidents_test.gd`, `smoke.gd`.

## Prochaine étape
Test headless, smoke, commit final.
