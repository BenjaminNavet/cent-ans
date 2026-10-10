# TW reinf — renforts différés (top5) et armée de secours de siège (top9)

Branche tw/reinf (base wr/int). ADR 0330.
- FAIT : sim-battle (`UnitSetup.arrival_s/entry_edge`, `reinforcements.rs`, `next_arrival_in`), tests `tests/combat/tw_reinf.rs`, données `reinforce_seconds_per_km` / `reinforce_base_delay_s`.
- RESTE : campagne (arrival_s par armée selon la distance, bord d'entrée, secours de siège), pont + HUD, ADR, lots.md.
