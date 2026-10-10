# TW pursuit (ADR 0321)

État : démarrage. Plan : `data/rules/battle_outcome.json` (+ `pursuit`, `unit_xp`, `spoils`), `SideResult.captured/pursuit_losses/unit_xp`
(serde default), calcul pur dans `sim-battle` (`pursuit.rs`), application dans `sim-campaign` (`battle_request.rs`), écran de fin.
Prochaine étape : data-model + calcul. Captifs de troupe : simple compteur (pas de rançon de troupe existante).
