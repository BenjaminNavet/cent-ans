# TW misc-camp (branche tw/misc-camp) — FAIT

ADR 0332. Marchand (`AgentKind::Merchant`, actions `trade_post`/`outbid`, revenu saisonnier `agents/merchants.rs`, IA `ai_merchant`) et objectifs de victoire étendus (conditions `province_count/province_growth/prestige/treasury/hold_title`, `scope` court/long, `short_end_year`, objectifs génériques dans `feudal.json` `victory`, 9 nouvelles factions, 4 historiques marquées).

Tests : `armies::tw_merchants` (8), `campaign_life::tw_victory` (5) ; `cargo test --workspace` vert (graines du test `cv3_ai_stances` passées à 3,4,5,6 : la graine 1 montre un déplacement refusé, point ouvert déjà connu).

Sonde `campaign_probe --turns 120 --seeds 1,2` (avant = marchand plafonné à 0 par `CENT_ANS_DATA_DIR`, après) :
- banqueroutes 115 / 124 -> 93 / 101 ; révoltes 2 / 2 -> 5 / 0 ; guerre FR-EN 68 / 83 % -> 64 / 77 % ; guerres actives 33,2 / 40,8 -> 34,3 / 39,8 ; éliminées 19 / 22 -> 26 / 24 ; 0,71 s/tour inchangé.
Pas de régression nette (bruit de trajectoire), moins de banqueroutes.

Restes : bouton court/long dans l'écran de nouvelle partie (le pont expose `set_victory_length`) ; vérification Godot non lancée (disque) : seules des tables de lettres/couleurs (`agent_controller.gd`, `encyclopedia.gd`) ont reçu `merchant` ; échelle du prestige à surveiller.
