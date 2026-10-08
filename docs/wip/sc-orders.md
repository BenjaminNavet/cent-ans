# SC orders (vague 7)

État : `orders.rs` découpé en `orders/{mod,order,error,recruit,army,build}.rs` ; recrutement via `RecruitContext` (effets, slots, prix calculés une fois par colonie) utilisé par l'ordre `Recruit`, le panneau, l'IA (`fill_site`, `reprice_recruits`), les missions ; réserves de recrutement (`recruit_pool_with`) sans recalcul d'effets par type d'unité.

Reste : vérifications finales (fmt, clippy, tests sim-campaign + ai).
