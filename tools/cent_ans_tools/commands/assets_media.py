"""Groupe ``assets`` (1/2) : héraldique, icônes, audio, portraits, illustrations."""

from __future__ import annotations

import typer
from rich.table import Table

from cent_ans_tools import blender
from cent_ans_tools.commands._common import console

assets_app = typer.Typer(
    help="Assets du jeu (héraldique, audio, modèles 3D, portraits).",
    no_args_is_help=True,
)


@assets_app.command("heraldry")
def assets_heraldry() -> None:
    """Dessine un écu PNG 128×128 par faction dans game/assets/heraldry/."""
    from cent_ans_tools import heraldry

    paths = heraldry.build()
    console.print(f"[green]OK[/green] : {len(paths)} écus dans {heraldry.HERALDRY_DIR}")
    houses = heraldry.build_houses()
    console.print(
        f"[green]OK[/green] : {len(houses)} écus de maison dans {heraldry.HOUSES_DIR}"
    )


@assets_app.command("banners")
def assets_banners() -> None:
    """Dessine bannières (256×512), pennons (512×128) et étendards (1024×256) de chaque faction."""
    from cent_ans_tools import banners

    paths = banners.build()
    console.print(f"[green]OK[/green] : {len(paths)} images dans {banners.BANNERS_DIR}")


@assets_app.command("icons")
def assets_icons(
    offline: bool = typer.Option(
        False, "--offline", help="Cache local seulement, aucun accès réseau"
    ),
) -> None:
    """Icônes game-icons.net (CC BY 3.0) teintées encre sépia dans game/assets/icons/."""
    from cent_ans_tools import icons

    missing = icons.missing_icons()
    if missing:
        console.print(f"[red]Icônes manquantes[/red] : {', '.join(missing)}")
        raise typer.Exit(code=1)
    rows, written = icons.build(offline=offline)
    files = len({row.file for row in rows})
    console.print(
        f"[green]OK[/green] : {len(rows)} identifiants, {files} SVG, "
        f"{written} fichier(s) écrit(s) dans {icons.ICONS_DIR}"
    )


@assets_app.command("ink-icons")
def assets_ink_icons(
    kind: str = typer.Option(
        "all", "--kind", help="icon, medallion ou all (les deux, icônes d'abord)"
    ),
    only: list[str] = typer.Option(
        None, "--only", help="Identifiants à générer seuls (sonde), répétable"
    ),
    limit: int | None = typer.Option(None, "--limit", help="Nombre maximal d'images"),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    build_only: bool = typer.Option(
        False, "--build-only", help="Aucune génération : dérive les PNG des sources"
    ),
    envelope: float | None = typer.Option(
        None, "--envelope", help="Enveloppe (défaut : reste du plafond du lot)"
    ),
    group: str | None = typer.Option(
        None,
        "--group",
        help='Ne traite que les entrées de ce groupe du catalogue (ex. "trait", DA7c)',
    ),
    subject: str | None = typer.Option(
        None,
        "--subject",
        help="Préfixe de ligne du grand livre et de plafond (défaut : le lot DA5)",
    ),
    budget_cap: float | None = typer.Option(
        None,
        "--budget-cap",
        help="Plafond du lot en dollars (défaut : budget_cap_usd du catalogue, lot DA5)",
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """DA5 : icônes d'action à l'encre et boutons-médaillons (une image par icône).

    Le même catalogue et le même outil couvrent d'autres lots (DA7c : icônes de trait,
    groupe ``trait``) via ``--group``/``--subject``/``--budget-cap``, avec leur propre
    enveloppe et leur propre ligne de grand livre plutôt que le plafond DA5 déjà dépensé.
    """
    from decimal import Decimal

    from cent_ans_tools import ink_icons, local_art
    from cent_ans_tools.budget import BudgetLedger

    catalog = ink_icons.load_catalog()
    model = local_art.MODEL_ID if local else catalog["model"]
    budget_session = catalog.get("budget_session")
    subject_line = subject or ink_icons.BUDGET_SUBJECT
    prefix = subject_line.split(" :")[0] + " :"
    lot_label = prefix.rstrip(" :")
    if not build_only:
        cap = (
            Decimal(str(budget_cap))
            if budget_cap is not None
            else Decimal(str(catalog["budget_cap_usd"]))
        )
        spent = ink_icons.lot_spent(BudgetLedger(), prefix)
        remaining = cap - spent
        if envelope is not None:
            remaining = min(remaining, Decimal(str(envelope)))
        console.print(
            f"Lot {lot_label} : {spent:.2f} $ déjà dépensés, enveloppe {remaining:.2f} $"
        )
        kinds = ["icon", "medallion"] if kind == "all" else [kind]
        for current in kinds:
            jobs = ink_icons.plan(
                catalog, kind=current, group=group, only=only or None, limit=limit
            )
            convert = (
                ink_icons.to_raw_icon
                if current == "icon"
                else ink_icons.to_raw_medallion
            )
            before = ink_icons.lot_spent(BudgetLedger(), prefix)
            _run_art_batch(
                jobs,
                model,
                float(remaining),
                dry_run,
                f"image(s) ({current})",
                subject_line,
                convert,
                budget_session=budget_session,
                aspect_ratio="1:1",
            )
            remaining -= ink_icons.lot_spent(BudgetLedger(), prefix) - before
        if dry_run:
            return
    report = ink_icons.build(catalog)
    sheet = ink_icons.contact_sheet(catalog)
    console.print(
        f"[green]OK[/green] : {len(report['icons'])} icône(s), "
        f"{len(report['medallions'])} médaillon(s), "
        f"{len(report.get('cursors', []))} curseur(s) ; planche {sheet}"
    )
    if report["missing"]:
        console.print(f"[yellow]Sans source[/yellow] : {', '.join(report['missing'])}")


@assets_app.command("entity-icons")
def assets_entity_icons(
    only: list[str] = typer.Option(
        None, "--only", help="Identifiants à générer seuls (sonde), répétable"
    ),
    limit: int | None = typer.Option(None, "--limit", help="Nombre maximal d'images"),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    build_only: bool = typer.Option(
        False,
        "--build-only",
        help="Aucune génération : dérive et encadre les miniatures",
    ),
    envelope: float | None = typer.Option(
        None, "--envelope", help="Enveloppe (défaut : reste du plafond du lot DA5b)"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """DA5b : icônes d'entité en miniatures peintes (dérivées des illustrations, sinon générées)."""
    from decimal import Decimal

    from cent_ans_tools import entity_icons, local_art
    from cent_ans_tools.budget import BudgetLedger

    catalog = entity_icons.load_catalog()
    if not build_only:
        spent = entity_icons.lot_spent(BudgetLedger())
        remaining = Decimal(str(catalog["budget_cap_usd"])) - spent
        if envelope is not None:
            remaining = min(remaining, Decimal(str(envelope)))
        console.print(
            f"Lot DA5b : {spent:.2f} $ déjà dépensés, enveloppe {remaining:.2f} $"
        )
        jobs = entity_icons.plan(catalog, only=only or None, limit=limit)
        _run_art_batch(
            jobs,
            local_art.MODEL_ID if local else catalog["model"],
            float(remaining),
            dry_run,
            "miniature(s)",
            entity_icons.BUDGET_SUBJECT,
            entity_icons.to_raw,
            aspect_ratio="1:1",
        )
        if dry_run:
            return
    report = entity_icons.build(catalog)
    sheet = entity_icons.contact_sheet(catalog)
    console.print(
        f"[green]OK[/green] : {len(report['derived'])} dérivée(s), "
        f"{len(report['generated'])} générée(s) ; planche {sheet}"
    )
    if report["missing"]:
        console.print(f"[yellow]Sans source[/yellow] : {', '.join(report['missing'])}")


@assets_app.command("unit-emblems")
def assets_unit_emblems() -> None:
    """OMR R5 : sources locales (emblème doré sur azur) des miniatures d'unités sans illustration."""
    from cent_ans_tools import unit_emblems

    paths = unit_emblems.build()
    console.print(
        f"[green]OK[/green] : {len(paths)} emblème(s) dans {unit_emblems.OUT_DIR} ; "
        "encadrer ensuite avec « assets entity-icons --build-only »"
    )


@assets_app.command("menu-art")
def assets_menu_art() -> None:
    """Dessine l'illustration du menu (carte ancienne 2560×1440) dans game/assets/ui/."""
    from cent_ans_tools import menu_art

    path = menu_art.build()
    console.print(
        f"[green]OK[/green] : {path} ({path.stat().st_size / 1e6:.1f} Mo) + {menu_art.SIDECAR_NAME}"
    )


@assets_app.command("ui-illumination")
def assets_ui_illumination() -> None:
    """Peint le kit d'UI enluminé (textures 9-slice) dans game/assets/ui/illumination/."""
    from cent_ans_tools import ui_illumination

    paths = ui_illumination.build()
    console.print(
        f"[green]OK[/green] : {len(paths)} texture(s) dans {ui_illumination.OUTPUT_DIR}"
    )


@assets_app.command("audio")
def assets_audio(
    no_music: bool = typer.Option(False, "--no-music", help="Effets seulement"),
) -> None:
    """Synthétise les effets (sfx/) et les 3 musiques modales (music/) en OGG ou WAV."""
    from cent_ans_tools import audio

    paths = audio.build(music=not no_music)
    total = sum(path.stat().st_size for path in paths) / 1e6
    console.print(
        f"[green]OK[/green] : {len(paths)} fichiers ({total:.1f} Mo) dans {audio.AUDIO_DIR}"
    )


@assets_app.command("models")
def assets_models() -> None:
    """Construit les modèles low-poly (Blender headless) dans game/assets/models/*.glb."""
    counts = blender.build_models()
    table = Table(title="Modèles glTF")
    table.add_column("Modèle")
    table.add_column("Triangles", justify="right")
    for name, triangles in counts.items():
        table.add_row(name, str(triangles))
    console.print(table)


@assets_app.command("portraits")
def assets_portraits(
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal de portraits"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : le moins cher retenu)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """Génère les portraits manquants (256×256) dans game/assets/portraits/."""
    from decimal import Decimal

    from cent_ans_tools import local_art, portraits

    model = local_art.MODEL_ID if local else model or portraits.DEFAULT_MODEL
    jobs = portraits.plan(limit=limit)
    if dry_run:
        for job in jobs:
            console.rule(job.character_id)
            console.print(job.prompt)
        unit = Decimal("0") if local else portraits.KNOWN_PRICES.get(model)
        cost = (
            f"≈ {unit * len(jobs):.2f} $"
            if unit is not None
            else "tarif inconnu hors ligne"
        )
        console.print(
            f"{len(jobs)} portrait(s) avec {model} : {cost} (aucun appel réseau)"
        )
        return
    if not jobs:
        console.print("[green]OK[/green] : tous les portraits existent déjà")
        return
    result = portraits.generate(
        jobs,
        model,
        envelope=Decimal(str(envelope)),
        on_progress=lambda job, spent: console.print(
            f"{job.character_id} ({spent:.4f} $)"
        ),
        image_config={"aspect_ratio": "3:4"} if local else None,
    )
    console.print(
        f"[green]OK[/green] : {len(result.written)} portrait(s), estimé {result.estimated:.4f} $, réel {result.actual:.4f} $"
    )


@assets_app.command("portrait-archetypes")
def assets_portrait_archetypes(
    limit: int | None = typer.Option(None, "--limit", help="Nombre maximal d'images"),
    only: list[str] = typer.Option(
        None, "--only", help="Clés à générer seules (sonde), répétable"
    ),
    no_aged: bool = typer.Option(False, "--no-aged", help="Archétypes seulement"),
    no_archetypes: bool = typer.Option(
        False, "--no-archetypes", help="Variantes âgées seulement"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        8.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """DA2 : archétypes de portraits et variantes âgées (512×512 JPEG)."""
    from cent_ans_tools import local_art, portrait_archetypes, portraits

    model = local_art.MODEL_ID if local else model or portraits.DEFAULT_MODEL
    jobs = portrait_archetypes.plan(
        archetypes=not no_archetypes,
        aged=not no_aged,
        only=only or None,
        limit=limit,
    )
    _run_art_batch(
        jobs,
        model,
        envelope,
        dry_run,
        "portrait(s) vivant(s)",
        "DA2 : portraits vivants (archétypes et variantes âgées)",
        portrait_archetypes.to_archetype_jpg,
        aspect_ratio="3:4",
    )


def _run_art_batch(
    jobs,
    model,
    envelope,
    dry_run,
    noun,
    subject,
    convert,
    budget_session=None,
    aspect_ratio=None,
) -> None:
    """Dry-run listing or paid batch for an image job list (event art, illustrations).

    ``budget_session`` names the `docs/budget.md` heading that funds this batch, when it is
    not necessarily the last table in the file (see `portraits.generate`).
    ``aspect_ratio`` sizes the free local model's output (``--model local/z-image-turbo``,
    ADR 0190); paid models keep their own default.
    """
    from decimal import Decimal

    from cent_ans_tools import local_art, portraits

    if dry_run:
        for job in jobs:
            console.rule(job.character_id)
            console.print(job.prompt)
        unit = (
            Decimal("0")
            if model == local_art.MODEL_ID
            else portraits.KNOWN_PRICES.get(model)
        )
        cost = (
            f"≈ {unit * len(jobs):.2f} $"
            if unit is not None
            else "tarif inconnu hors ligne"
        )
        console.print(f"{len(jobs)} {noun} avec {model} : {cost} (aucun appel réseau)")
        return
    if not jobs:
        console.print(f"[green]OK[/green] : aucune {noun} manquante")
        return
    result = portraits.generate(
        jobs,
        model,
        envelope=Decimal(str(envelope)),
        budget_session=budget_session,
        subject=f"{subject} ({len(jobs)} × {model})",
        convert=convert,
        on_progress=lambda job, spent: console.print(
            f"{job.character_id} ({spent:.4f} $)"
        ),
        image_config=(
            {"aspect_ratio": aspect_ratio}
            if aspect_ratio and model == local_art.MODEL_ID
            else None
        ),
    )
    console.print(
        f"[green]OK[/green] : {len(result.written)} {noun}, estimé {result.estimated:.4f} $, réel {result.actual:.4f} $"
    )


@assets_app.command("event-art")
def assets_event_art(
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'illustrations"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """Génère les miniatures d'événements manquantes (768×432) dans game/assets/events/."""
    from cent_ans_tools import event_art, local_art, portraits

    _run_art_batch(
        event_art.plan(limit=limit),
        local_art.MODEL_ID if local else model or portraits.DEFAULT_MODEL,
        envelope,
        dry_run,
        "miniature(s)",
        "Miniatures d'événements",
        event_art.to_miniature_jpg,
        aspect_ratio="16:9",
    )


@assets_app.command("illustrations")
def assets_illustrations(
    category: str | None = typer.Option(
        None,
        "--category",
        help="unit_types, buildings, technologies, factions, séparées par des virgules (défaut : toutes)",
    ),
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'illustrations"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """Génère les miniatures de l'encyclopédie (640×360) dans game/assets/illustrations/."""
    from cent_ans_tools import entry_art, local_art, portraits

    categories = (
        tuple(part.strip() for part in category.split(","))
        if category
        else entry_art.CATEGORIES
    )
    _run_art_batch(
        entry_art.plan(categories=categories, limit=limit),
        local_art.MODEL_ID if local else model or portraits.DEFAULT_MODEL,
        envelope,
        dry_run,
        "illustration(s)",
        "Illustrations de l'encyclopédie",
        entry_art.convert,
        aspect_ratio="16:9",
    )


@assets_app.command("horizon-panoramas")
def assets_horizon_panoramas(
    ids: str = typer.Option(
        "", "--ids", help="Panoramas à générer (virgules ; défaut : tous)"
    ),
    generate: bool = typer.Option(
        False, "--generate", help="Appels payants OpenRouter (sinon traitement seul)"
    ),
    dry_run: bool = typer.Option(False, "--dry-run", help="Affiche les invites"),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """Panoramas d'horizon peints des batailles (EP2) : génération puis détourage."""
    from cent_ans_tools import horizon_panoramas, local_art

    data = horizon_panoramas.load_data()
    wanted = [i.strip() for i in ids.split(",") if i.strip()] or list(data["panoramas"])
    if generate or dry_run:
        horizon_panoramas.generate(
            wanted,
            local_art.MODEL_ID if local else horizon_panoramas.DEFAULT_MODEL,
            dry_run=dry_run,
            data=data,
        )
    if not dry_run:
        meta = horizon_panoramas.process_all(data)
        console.print(f"{len(meta['panoramas'])} panoramas traités")


# Local (mflux) aspect per plate family, closest to the final crop (ADR 0190).
ART_PLATE_ASPECTS = {"loading": "16:9", "vignette": "21:9", "ending": "16:9"}


@assets_app.command("art-plates")
def assets_art_plates(
    generate: bool = typer.Option(
        False, "--generate", help="Génère aussi les manques (appel payant OpenRouter)"
    ),
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'images générées"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    recrop: bool = typer.Option(
        False, "--recrop", help="Recadre les planches Commons existantes (cache local)"
    ),
    envelope: float = typer.Option(
        8.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """AR1 : planches illustrées (chargements, vignettes, fins) dans game/assets/art/."""
    from cent_ans_tools import art_plates, local_art, portraits

    written = art_plates.build_commons(force=recrop)
    console.print(f"Commons : {len(written)} planche(s) écrite(s)")
    if not (generate or dry_run):
        return
    jobs = art_plates.generation_jobs()[:limit]
    for family in art_plates.FORMATS:
        family_jobs = [job for job in jobs if art_plates.family_of(job) == family]
        if not family_jobs:
            continue
        _run_art_batch(
            family_jobs,
            local_art.MODEL_ID if local else portraits.DEFAULT_MODEL,
            envelope,
            dry_run,
            "planche(s)",
            f"AR1 : planches illustrées ({family})",
            lambda image, family=family: art_plates.convert_generated(image, family),
            aspect_ratio=ART_PLATE_ASPECTS[family],
        )


@assets_app.command("codex-art")
def assets_codex_art(
    category: str | None = typer.Option(
        None,
        "--category",
        help="Catégories du Codex séparées par des virgules (défaut : toutes sauf personnages et plantes)",
    ),
    limit: int | None = typer.Option(
        None, "--limit", help="Nombre maximal d'illustrations"
    ),
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Affiche prompts et coût, sans appel payant"
    ),
    model: str = typer.Option(
        None, "--model", help="Modèle OpenRouter (défaut : celui des portraits)"
    ),
    envelope: float = typer.Option(
        10.0, "--envelope", help="Enveloppe maximale de ce lot en dollars"
    ),
    local: bool = typer.Option(
        False, "--local", help="Génération locale gratuite (mflux, ADR 0190)"
    ),
) -> None:
    """Génère les miniatures des fiches du Codex sans image d'entité (640×360)."""
    from cent_ans_tools import codex_art, local_art, portraits

    categories = (
        tuple(part.strip() for part in category.split(","))
        if category
        else codex_art.CATEGORIES
    )
    _run_art_batch(
        codex_art.plan(categories=categories, limit=limit),
        local_art.MODEL_ID if local else model or portraits.DEFAULT_MODEL,
        envelope,
        dry_run,
        "illustration(s)",
        "Illustrations du Codex",
        codex_art.convert,
        aspect_ratio="16:9",
    )
