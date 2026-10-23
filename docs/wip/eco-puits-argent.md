# ECO — puits d'argent de la France

Constat du juge (out/juge/iter-02.md C2) : trésor France 28 000 -> 111 667 en 4 saisons passives, net +16 à +20 k par saison. ADR 0749.

## État
- [x] `campaign_probe --passive` (série `france_treasury` en JSON).
- [x] Avant (40 tours passifs, graines 1-2) : pic trésor France 207-226 k ; tour 40 : 136-152 k ; banqueroutes 1 / 2.
- [x] Après : paliers d'épargne (2 saisons 15 %, 4 saisons +20 %, 8 saisons +25 %, `opulence_tiers`) : pic 135-136 k (6,8 saisons de solde) ; tour 40 : 81-83 k ; banqueroutes 4 / 5 (toutes factions, 40 tours).
- [x] Effets de bâtiments logistique/armée : supply (corn_hall, tithe_barn, royal_road), army_experience (muster_field), recruit_slots (castle, artillery_bastion).
- [x] Tests ajustés : f4_war (trésor 150 k), jr_crusade (net hors ponction), b7a, nouveau `the_hoard_drain_is_progressive`.
- Workspace complet lancé une fois : seuls 2 échecs (corrigés ensuite) ; le correctif jr_crusade passe, `valid_orders_apply_immediately` passait après retrait du recrutement -5 % (cause : coût milice 300).

## Non fait / non vérifié
- Seed bots `--quick` non lancé ; clippy/fmt après les dernières retouches (jr_crusade.rs, bâtiments) non relancés ; test complet final à refaire par l'intégrateur.
- Pas de puits structurel (solde, dot, cadeaux) : net initial France reste +20 k.
- Numéro ADR 0749 à vérifier contre les autres lots.
