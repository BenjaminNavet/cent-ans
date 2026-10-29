# Lot MEM5 : ~3 Go de mémoire graphique non expliqués en campagne

Branche `worktree-agent-a319572925dedf007`, base main 99d672914. Suite de MEM4 (ADR 0794).

## Constat (mesuré, campagne au repos, release, Ultra, hors écran)
- Les « ~3 Go graphiques non comptés » sont le **tas mimalloc** (mémoire CPU) : mimalloc étiquette ses pages avec le
  tag VM 100 (`os_tag` par défaut), qui est `VM_MEMORY_IOACCELERATOR` ; `footprint`/`vmmap` les affichent donc comme
  `IOAccelerator` (sans « (graphics) »). Preuve : 51 régions de 128 Mio rw- (arènes mimalloc) = 2 664 Mo ; avec
  `--features system-alloc`, la catégorie `IOAccelerator` tombe à 1,2 Mo et `Malloc Large/Small` prend le relais.
- Décomposition de l'empreinte (5 790 Mo, t+10 s) : GPU non mappé 2 732 (≈ Metal alloué 2 492 : images 1 995 +
  ≈ 500 maillages/cibles) ; tas mimalloc 2 664 ; GPU mappé 168 ; reste ≈ 230.
- Contenu du tas (`MallocStackLogging=lite` + `malloc_history -allBySize`, binaire system-alloc, 3 078 Mio vivants) :
  végétation 920 (atlas d'imposteurs 2 x 165 en copie CPU après les mips, rasters 7080x6104 du semis 590), relief E0
  124, ids de provinces client 82 + raster provinces/navgrid du cœur 82, polices 89, copies CPU des glTF 89, ECS 79,
  masque terre décodé 2 fois (48 + 41), countryside 92, coast rocks 41, LifeData 43, warp 42, rubans d'eau 64...

## Fait (non mesuré encore)
- Atlas d'imposteurs : `RENDER_WORLD` à l'installation de la chaîne de mips (-330 Mo attendus).
- `wetlands.png` du semis : un plan max(R, G) au lieu de RGB (-86 Mo).
- `land_mask.png` : décodé une fois (`shared_land_mask`, cache `Weak`) pour les ordres et l'eau douce (-43 Mo).
- `mem_protocol.sh` : colonnes séparées GPU non mappé / GPU mappé / tas mimalloc.

## Prochaine étape
1. Mesurer (release, `mem_protocol.sh idle`) avant/après ; vérifier le rendu (1 capture) de la végétation lointaine.
2. ADR, `docs/performance-memoire.md`, seuils.

## Points ouverts
- Doublons restants : rasters provinces client/cœur (82 Mo), `river_bed` x3, octets de hauteur du semis (86 Mo,
  `MapRasters.height` du cœur).
