"""Live local gallery of the DN night batch: generated images and 3D models as they arrive.

Serves ~/dev/cent-ans-raw/dn on http://127.0.0.1:8765 and builds the index page on every request,
newest object first. The page reloads itself when a file changes.

Usage: uv run python tools/experiments/dn_live_gallery.py [--port 8765] [--root ~/dev/cent-ans-raw/dn]
"""

import argparse
import functools
import html
import json
import os
import urllib.parse
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

MODEL_VIEWER = "https://unpkg.com/@google/model-viewer@3.5.0/dist/model-viewer.min.js"


OUTPUT_MARKERS = ("prompt.txt", "img", "cut", "3d")
CATALOG_GLOB = "data/art/dn_catalog_*.json"
REPO_ROOT = Path(__file__).resolve().parents[2]


def object_dirs(root: Path) -> list[Path]:
    """Return per-object folders (any generation output), most recently modified first."""
    folders = [
        folder
        for folder in root.iterdir()
        if folder.is_dir()
        and any((folder / marker).exists() for marker in OUTPUT_MARKERS)
    ]
    return sorted(folders, key=latest_mtime, reverse=True)


def latest_mtime(folder: Path) -> float:
    """Return the newest modification time of any file under the folder."""
    times = [path.stat().st_mtime for path in folder.rglob("*") if path.is_file()]
    return max(times, default=folder.stat().st_mtime)


def catalog_prompts() -> dict[str, str]:
    """Return the prompt of every catalogue entry by id (fallback when prompt.txt is absent)."""
    prompts: dict[str, str] = {}
    for catalog_path in REPO_ROOT.glob(CATALOG_GLOB):
        try:
            entries = json.loads(catalog_path.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if isinstance(entries, dict):
            entries = next((v for v in entries.values() if isinstance(v, list)), [])
        for entry in entries:
            if isinstance(entry, dict) and "id" in entry:
                prompts[entry["id"]] = entry.get("prompt", "")
    return prompts


def charter_status(root: Path) -> dict[str, str]:
    """Return the last D5 charter status per object id."""
    statuses: dict[str, str] = {}
    log_path = root / "charter.jsonl"
    if log_path.exists():
        for line in log_path.read_text().splitlines():
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue
            statuses[entry.get("id", "")] = entry.get("status", "")
    return statuses


def object_card(
    root: Path, folder: Path, statuses: dict[str, str], prompts: dict[str, str]
) -> str:
    """Render one object: prompt, image attempts, chosen seed, 3D sheet and viewers."""
    object_id = folder.name
    rel = folder.relative_to(root).as_posix()
    chosen_path = folder / "chosen.json"
    chosen_seed = (
        json.loads(chosen_path.read_text()).get("seed")
        if chosen_path.exists()
        else None
    )
    image_dir = folder / "cut" if (folder / "cut").exists() else folder / "img"
    images = sorted(image_dir.glob("*.png")) if image_dir.exists() else []
    thumbs = "".join(
        f'<figure class="{"chosen" if chosen_seed is not None and image.stem == f"s{chosen_seed}" else ""}">'
        f'<a href="/{rel}/{image_dir.name}/{image.name}"><img loading="lazy" src="/{rel}/{image_dir.name}/{image.name}"></a>'
        f"<figcaption>{image.stem}</figcaption></figure>"
        for image in images
    )
    sheet = (
        f'<a href="/{rel}/sheet.png"><img class="sheet" loading="lazy" src="/{rel}/sheet.png"></a>'
        if (folder / "sheet.png").exists()
        else ""
    )
    models = sorted((folder / "3d").glob("*.glb")) if (folder / "3d").exists() else []
    viewers = "".join(
        f'<figure><model-viewer src="/{rel}/3d/{model.name}" camera-controls auto-rotate shadow-intensity="1" '
        f'exposure="1.1"></model-viewer><figcaption>{model.stem}</figcaption></figure>'
        for model in models
    )
    status = statuses.get(object_id, "")
    stage = (
        "3D"
        if models
        else (
            "détouré"
            if (folder / "cut").exists()
            else ("images" if images else "en attente")
        )
    )
    prompt_path = folder / "prompt.txt"
    prompt_text = (
        prompt_path.read_text() if prompt_path.exists() else prompts.get(object_id, "")
    )
    prompt = html.escape(prompt_text)
    generation_path = folder / "generation.json"
    generation = (
        json.loads(generation_path.read_text()) if generation_path.exists() else {}
    )
    origin = html.escape(
        " · ".join(
            str(generation[key])
            for key in ("catalogue", "target", "region")
            if generation.get(key)
        )
    )
    links = " ".join(
        f'<a href="/{rel}/{name}">{name}</a>'
        for name in ("prompt.txt", "generation.json")
        if (folder / name).exists()
    )
    view_images = (
        sorted((folder / "views").glob("*.png")) if (folder / "views").exists() else []
    )
    views = "".join(
        f'<figure><a href="/{rel}/views/{image.name}"><img loading="lazy" src="/{rel}/views/{image.name}"></a>'
        f"<figcaption>vue {image.stem}</figcaption></figure>"
        for image in view_images
    )
    search_text = html.escape(f"{object_id} {origin} {prompt_text}".lower(), quote=True)
    return (
        f'<section id="{object_id}" data-search="{search_text}"><h2><a href="#{object_id}">{object_id}</a> <span class="tag">{stage}</span>'
        f"{f'<span class=tag>{status}</span>' if status else ''}</h2>"
        f'<p class="origin">{origin} {links}</p><p class="prompt">{prompt}</p>'
        f'<div class="row">{thumbs}{views}</div>{sheet}<div class="row">{viewers}</div></section>'
    )


IMAGE_SUFFIXES = (".png", ".jpg", ".jpeg", ".webp")
CAPTION_FIELDS = (
    ("lieu", "Lieu"),
    ("distance", "Distance caméra"),
    ("regarder", "À regarder"),
    ("avant_apres", "Avant / après"),
)


def chantier_dirs(root: Path) -> list[Path]:
    """Return the chantier folders (``<root>/chantiers/*``), oldest first (date, else mtime)."""
    base = root / "chantiers"
    if not base.is_dir():
        return []
    return sorted(
        (folder for folder in base.iterdir() if folder.is_dir()),
        key=lambda folder: (
            chantier_meta(folder).get("date", ""),
            latest_mtime(folder),
        ),
    )


def chantier_meta(folder: Path) -> dict:
    """Read the optional ``_chantier.json`` (titre, date, resume) of a chantier folder."""
    try:
        return json.loads((folder / "_chantier.json").read_text())
    except (OSError, ValueError):
        return {}


def read_captions(folder: Path) -> dict:
    """Read ``captions.json`` (file name to caption: a string or an object of fields)."""
    try:
        data = json.loads((folder / "captions.json").read_text())
    except (OSError, ValueError):
        return {}
    return data if isinstance(data, dict) else {}


def caption_html(caption: object) -> str:
    """Render one caption (plain string or object with titre/lieu/distance/regarder/...)."""
    if isinstance(caption, str):
        return f"<p>{html.escape(caption)}</p>"
    if not isinstance(caption, dict):
        return ""
    parts = [f"<h3>{html.escape(str(caption.get('titre', '')))}</h3>"]
    for key, label in CAPTION_FIELDS:
        if caption.get(key):
            parts.append(f"<p><b>{label}</b> : {html.escape(str(caption[key]))}</p>")
    return "".join(parts)


def chantier_section(folder: Path) -> str:
    """Render one chantier: its title and its captioned captures."""
    meta = chantier_meta(folder)
    captions = read_captions(folder)
    images = sorted(
        path.name for path in folder.iterdir() if path.suffix.lower() in IMAGE_SUFFIXES
    )
    figures = "".join(
        f'<figure><a href="/chantiers/{html.escape(folder.name)}/{html.escape(name)}">'
        f'<img src="/chantiers/{html.escape(folder.name)}/{html.escape(name)}" loading="lazy"></a>'
        f"<figcaption>{caption_html(captions.get(name, name))}</figcaption></figure>"
        for name in images
    )
    title = html.escape(str(meta.get("titre", folder.name)))
    date = html.escape(str(meta.get("date", "")))
    summary = html.escape(str(meta.get("resume", "")))
    return (
        f'<section id="{html.escape(folder.name)}"><h2>{title} <span class=tag>{len(images)} captures</span>'
        f'<span class=tag>{date}</span></h2><p class="origin">{summary}</p>'
        f'<div class="shots">{figures}</div></section>'
    )


def render_chantiers(root: Path) -> str:
    """Build the « Chantiers » page: captures of each finished DN chantier, with captions."""
    folders = chantier_dirs(root)
    nav = " · ".join(
        f'<a href="#{html.escape(f.name)}">{html.escape(str(chantier_meta(f).get("titre", f.name)))}</a>'
        for f in folders
    )
    sections = "".join(chantier_section(folder) for folder in folders)
    return f"""<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Chantiers DN</title>
<style>
body{{background:#1d1b18;color:#e8e1d3;font:14px system-ui;margin:0;padding:16px}}
h1{{font-weight:600}} h2{{margin:0 0 4px;font-size:18px}} h3{{margin:2px 0;font-size:13px;color:#e8e1d3}}
section{{border-top:1px solid #444;padding:12px 0}}
.tag{{font-size:11px;background:#3a352d;border-radius:4px;padding:2px 6px;margin-left:8px}}
.origin{{font-size:12px;margin:2px 0;color:#a99f8c}} a{{color:#c9a227}}
.shots{{display:flex;flex-wrap:wrap;gap:12px}}
figure{{margin:0;width:min(620px,100%);font-size:12px;color:#a99f8c}}
figure img{{width:100%;background:#000;border-radius:4px}}
figcaption p{{margin:2px 0}} figcaption b{{color:#e8e1d3}}
</style></head><body><h1>Chantiers DN : captures à juger ({len(folders)} chantiers)</h1>
<p><a href="/">Retour à la galerie des objets</a></p><p>{nav}</p>
{sections or "<p>Aucun chantier : déposer des captures et un captions.json dans chantiers/&lt;chantier&gt;/.</p>"}
</body></html>"""


def serve_chantier_image(
    handler: SimpleHTTPRequestHandler, root: Path, route: str
) -> None:
    """Serve /chantiers/<chantier>/<image> only if it resolves inside the chantiers folder."""
    parts = route.split("/")
    base = (root / "chantiers").resolve()
    if len(parts) != 4 or not all(parts[2:]):
        handler.send_error(404)
        return
    target = (base / parts[2] / parts[3]).resolve()
    if (
        target.parent.parent != base
        or target.suffix.lower() not in IMAGE_SUFFIXES
        or not target.is_file()
    ):
        handler.send_error(404)
        return
    payload = target.read_bytes()
    handler.send_response(200)
    handler.send_header(
        "Content-Type",
        "image/jpeg"
        if target.suffix.lower() in (".jpg", ".jpeg")
        else f"image/{target.suffix.lower()[1:]}",
    )
    handler.send_header("Content-Length", str(len(payload)))
    handler.end_headers()
    handler.wfile.write(payload)


def render_index(root: Path) -> str:
    """Build the whole gallery page."""
    statuses = charter_status(root)
    prompts = catalog_prompts()
    folders = object_dirs(root)
    cards = "".join(object_card(root, folder, statuses, prompts) for folder in folders)
    return f"""<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Galerie DN</title>
<script type="module" src="{MODEL_VIEWER}"></script>
<style>
body{{background:#1d1b18;color:#e8e1d3;font:14px system-ui;margin:0;padding:16px}}
h1{{font-weight:600}} h2{{margin:0 0 4px;font-size:18px}}
section{{border-top:1px solid #444;padding:12px 0}}
.tag{{font-size:11px;background:#3a352d;border-radius:4px;padding:2px 6px;margin-left:8px}}
.prompt{{color:#a99f8c;font-size:12px;max-width:900px;user-select:all}}
.origin{{font-size:12px;margin:2px 0}} a{{color:#c9a227}}
#q{{width:min(600px,100%);padding:6px 8px;font-size:14px;background:#2b2823;color:#e8e1d3;border:1px solid #555;border-radius:4px}}
.row{{display:flex;flex-wrap:wrap;gap:8px}}
figure{{margin:0;text-align:center;font-size:11px;color:#a99f8c}}
figure img{{width:200px;height:200px;object-fit:contain;background:#fff;border-radius:4px}}
figure.chosen img{{outline:3px solid #c9a227}}
img.sheet{{max-width:100%;margin:8px 0;border-radius:4px}}
model-viewer{{width:320px;height:320px;background:#2b2823;border-radius:4px}}
</style></head><body><h1>Galerie DN — production de la nuit ({len(folders)} objets)</h1>
<p><a href="/chantiers"><b>Chantiers : captures par chantier terminé</b></a></p>
<p>Objet le plus récent en haut. Cadre doré = essai retenu. La page se recharge seule quand un fichier change.
Chaque objet : catalogue, cible en jeu, prompt complet (clic = sélection), fiche <code>generation.json</code>.</p>
<input id="q" placeholder="Rechercher (id, catalogue, mot du prompt)…"> <span id="n"></span>
{cards}
<script>
const q=document.getElementById('q'),n=document.getElementById('n');
function filt(){{const t=q.value.toLowerCase().trim();let k=0;
document.querySelectorAll('section').forEach(e=>{{const on=!t||e.dataset.search.includes(t);e.style.display=on?'':'none';k+=on;}});
n.textContent=k+' affichés';try{{sessionStorage.setItem('q',q.value)}}catch(e){{}}}}
try{{q.value=sessionStorage.getItem('q')||''}}catch(e){{}}
q.addEventListener('input',filt);filt();
let stamp=null;
setInterval(async()=>{{try{{const r=await fetch('/__stamp');const s=await r.text();
if(stamp!==null&&s!==stamp)location.reload();stamp=s;}}catch(e){{}}}},20000);
</script></body></html>"""


class GalleryHandler(SimpleHTTPRequestHandler):
    """Static file handler over the batch folder with a dynamic index and change stamp."""

    def do_GET(self) -> None:  # noqa: D102 (http.server API)
        root = Path(self.directory)
        if self.path in ("/", "/index.html"):
            self.send_text(render_index(root), "text/html; charset=utf-8")
        elif self.path.split("?")[0] in ("/chantiers", "/chantiers/"):
            self.send_text(render_chantiers(root), "text/html; charset=utf-8")
        elif self.path.startswith("/chantiers/"):
            serve_chantier_image(
                self, root, urllib.parse.unquote(self.path.split("?")[0])
            )
        elif self.path == "/__stamp":
            self.send_text(
                str(max((latest_mtime(f) for f in object_dirs(root)), default=0)),
                "text/plain",
            )
        else:
            super().do_GET()

    def send_text(self, body: str, content_type: str) -> None:
        """Send a generated text response."""
        payload = body.encode()
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, format: str, *args: object) -> None:  # noqa: A002, D102 (silence access log)
        pass


def main() -> None:
    """Parse arguments and serve the gallery."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument(
        "--root", type=Path, default=Path(os.path.expanduser("~/dev/cent-ans-raw/dn"))
    )
    arguments = parser.parse_args()
    handler = functools.partial(GalleryHandler, directory=str(arguments.root))
    print(f"Galerie DN : http://127.0.0.1:{arguments.port}/")
    ThreadingHTTPServer(("127.0.0.1", arguments.port), handler).serve_forever()


if __name__ == "__main__":
    main()
