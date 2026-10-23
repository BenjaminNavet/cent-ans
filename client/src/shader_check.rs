//! Garde-fou : tout shader `.wesl` du client doit se compiler (wesl, imports réels de Bevy lus dans
//! le registre cargo) puis se valider (naga). Une erreur de shader (binding non déclaré, import
//! cassé) ne fait échouer que le pipeline au lancement du jeu : le sol devient noir sans test
//! rouge. Ce test la rend rouge. Voir ADR 0774.
#![cfg(test)]

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use wesl::syntax::{ModulePath, PathOrigin};
use wesl::{CompileOptions, ResolveError, Resolver};

/// Constantes que Bevy injecte dans le module virtuel `constants`.
const CONSTANTS: &str = "const MATERIAL_BIND_GROUP = 2;\n";

/// Résout les modules de Bevy (sources du registre cargo) et ceux du client.
struct RepoResolver {
    roots: Vec<(String, PathBuf)>,
}

impl RepoResolver {
    fn new() -> Self {
        let home = std::env::var("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap()).join(".cargo"));
        let registry = std::fs::read_dir(home.join("registry/src"))
            .expect("registre cargo")
            .flatten()
            .map(|e| e.path())
            .collect::<Vec<_>>();
        let mut roots = vec![(
            "cent_ans_client".to_string(),
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        )];
        for krate in [
            "bevy_pbr",
            "bevy_render",
            "bevy_core_pipeline",
            "bevy_ui_render",
            "bevy_mesh",
            "bevy_light",
            "bevy_camera",
        ] {
            let prefix = format!("{krate}-0.20");
            for dir in &registry {
                if let Some(found) = std::fs::read_dir(dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|e| e.path())
                    .find(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with(&prefix))
                    })
                {
                    roots.push((krate.to_string(), found.join("src")));
                }
            }
        }
        Self { roots }
    }
}

impl Resolver for RepoResolver {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        let PathOrigin::Package(pkg) = &path.origin else {
            return Err(ResolveError::ModuleNotFound(path.clone(), "origine".into()));
        };
        if pkg == "constants" {
            return Ok(Cow::Borrowed(CONSTANTS));
        }
        let rel: PathBuf = path.components.iter().map(|c| c.to_string()).collect();
        for (name, root) in &self.roots {
            if name == pkg {
                let file = root.join(&rel).with_extension("wesl");
                if let Ok(src) = std::fs::read_to_string(&file) {
                    return Ok(Cow::Owned(src));
                }
            }
        }
        Err(ResolveError::ModuleNotFound(
            path.clone(),
            "introuvable dans les sources du client ni de Bevy".into(),
        ))
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "wesl") {
            out.push(path);
        }
    }
}

/// Compile et valide un shader sous un jeu de drapeaux ; renvoie le message d'erreur éventuel.
fn check(resolver: &RepoResolver, module: &ModulePath, flags: &[&str]) -> Result<(), String> {
    let mut options = CompileOptions {
        imports: true,
        condcomp: true,
        visibility: false,
        ..Default::default()
    };
    for flag in flags {
        options
            .features
            .flags
            .insert((*flag).to_string(), true.into());
    }
    let compiled = wesl::compile(module, &options, resolver)
        .map_err(|e| e.diagnostic().render_plain().to_string())?;
    let wgsl = compiled.to_string();
    let parsed = naga::front::wgsl::parse_str(&wgsl).map_err(|e| e.emit_to_string(&wgsl))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&parsed)
    .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// Tous les `.wesl` du client se compilent et se valident, avec les drapeaux d'un maillage complet.
#[test]
fn every_client_shader_compiles_and_validates() {
    let resolver = RepoResolver::new();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect(&src, &mut files);
    assert!(files.len() > 30, "shaders introuvables ({})", files.len());
    let flags = [
        "VERTEX_NORMALS",
        "VERTEX_UVS",
        "VERTEX_UVS_A",
        "VERTEX_COLORS",
    ];
    let mut failures = Vec::new();
    for file in &files {
        let rel = file.strip_prefix(&src).unwrap().with_extension("");
        let module = ModulePath {
            origin: PathOrigin::Package("cent_ans_client".to_string()),
            components: rel
                .iter()
                .map(|c| c.to_string_lossy().to_string().into())
                .collect(),
        };
        if let Err(e) = check(&resolver, &module, &flags) {
            failures.push(format!("{}:\n{e}", file.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "{} shader(s) invalide(s) sur {}:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n---\n")
    );
}
