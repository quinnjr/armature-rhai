//! Engine hardening shared by every Armature-managed Rhai engine.
//!
//! A stock [`rhai::Engine::new()`] keeps two capabilities that a web-facing
//! scripting host should not hand out by default:
//!
//! - the `eval` function, which compiles and runs arbitrary strings (so any
//!   request data that reaches it becomes code), and
//! - a [`FileModuleResolver`] rooted at the process working directory, so
//!   `import "/etc/whatever"` or `import "../../secrets"` reads any `.rhai`
//!   file the process can see.
//!
//! [`harden_engine`] removes `eval` and installs a resolver that refuses every
//! `import`. [`SandboxedModuleResolver`] is the opt-in alternative: imports
//! resolve only to files that canonicalize inside one root directory.

use rhai::module_resolvers::{DummyModuleResolver, FileModuleResolver};
use rhai::{AST, Engine, EvalAltResult, Module, ModuleResolver, Position, Shared};
use std::path::{Component, Path, PathBuf};

/// Remove `eval` and disable `import` on `engine`.
///
/// Call [`Engine::set_module_resolver`] with a [`SandboxedModuleResolver`]
/// afterwards to allow imports from one directory.
///
/// ```
/// use armature_rhai::harden_engine;
///
/// let mut engine = rhai::Engine::new();
/// harden_engine(&mut engine);
/// assert!(engine.compile(r#"eval("1 + 1")"#).is_err());
/// ```
pub fn harden_engine(engine: &mut Engine) {
    engine.disable_symbol("eval");
    engine.set_module_resolver(DummyModuleResolver::new());
}

/// A module resolver confined to one directory.
///
/// An import path must be relative and free of `..`, and the resolved file
/// (after following symlinks) must lie inside the root; anything else fails
/// with "module not found". Resolution otherwise matches
/// [`FileModuleResolver::new_with_path`] (the `.rhai` extension is added).
pub struct SandboxedModuleResolver {
    root: PathBuf,
    inner: FileModuleResolver,
}

impl SandboxedModuleResolver {
    /// Confine imports to `root`.
    ///
    /// Fails if `root` cannot be canonicalized (e.g. it does not exist).
    pub fn new(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().canonicalize()?;
        Ok(Self {
            inner: FileModuleResolver::new_with_path(&root),
            root,
        })
    }

    /// The canonical root directory imports are confined to.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn check(&self, path: &str, pos: Position) -> Result<(), Box<EvalAltResult>> {
        let not_found = || Box::new(EvalAltResult::ErrorModuleNotFound(path.to_string(), pos));

        if Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err(not_found());
        }

        let resolved = self.inner.get_file_path(path, None);
        match resolved.canonicalize() {
            Ok(canonical) if canonical.starts_with(&self.root) => Ok(()),
            _ => Err(not_found()),
        }
    }
}

impl ModuleResolver for SandboxedModuleResolver {
    fn resolve(
        &self,
        engine: &Engine,
        _source: Option<&str>,
        path: &str,
        pos: Position,
    ) -> Result<Shared<Module>, Box<EvalAltResult>> {
        self.check(path, pos)?;
        self.inner.resolve(engine, None, path, pos)
    }

    fn resolve_ast(
        &self,
        engine: &Engine,
        _source: Option<&str>,
        path: &str,
        pos: Position,
    ) -> Option<Result<AST, Box<EvalAltResult>>> {
        if let Err(e) = self.check(path, pos) {
            return Some(Err(e));
        }
        self.inner.resolve_ast(engine, None, path, pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hardened_engine_rejects_eval() {
        let mut engine = Engine::new();
        harden_engine(&mut engine);
        assert!(engine.compile(r#"eval("40 + 2")"#).is_err());
        assert!(engine.eval::<i64>(r#"eval("40 + 2")"#).is_err());
    }

    #[test]
    fn hardened_engine_rejects_imports() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(temp.path().join("m.rhai"), "export const X = 1;").unwrap();
        let abs = temp.path().join("m");

        let mut engine = Engine::new();
        harden_engine(&mut engine);
        let script = format!("import {:?} as m; m::X", abs.to_str().unwrap());
        assert!(engine.eval::<i64>(&script).is_err());
    }

    #[test]
    fn sandboxed_resolver_allows_files_inside_root() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::create_dir(temp.path().join("lib")).unwrap();
        fs::write(temp.path().join("lib/m.rhai"), "export const X = 7;").unwrap();

        let mut engine = Engine::new();
        harden_engine(&mut engine);
        engine.set_module_resolver(SandboxedModuleResolver::new(temp.path()).unwrap());
        assert_eq!(
            engine.eval::<i64>(r#"import "lib/m" as m; m::X"#).unwrap(),
            7
        );
    }

    #[test]
    fn sandboxed_resolver_rejects_escapes() {
        let outer = tempfile::TempDir::new().unwrap();
        let root = outer.path().join("root");
        fs::create_dir(&root).unwrap();
        fs::write(outer.path().join("secret.rhai"), "export const X = 1;").unwrap();

        let mut engine = Engine::new();
        harden_engine(&mut engine);
        engine.set_module_resolver(SandboxedModuleResolver::new(&root).unwrap());

        // Parent-directory traversal.
        assert!(
            engine
                .eval::<i64>(r#"import "../secret" as m; m::X"#)
                .is_err()
        );
        // Absolute path.
        let abs = outer.path().join("secret");
        let script = format!("import {:?} as m; m::X", abs.to_str().unwrap());
        assert!(engine.eval::<i64>(&script).is_err());

        // Symlink pointing outside the root.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outer.path().join("secret.rhai"), root.join("link.rhai"))
                .unwrap();
            assert!(engine.eval::<i64>(r#"import "link" as m; m::X"#).is_err());
        }
    }
}
