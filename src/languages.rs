use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Language information with byte count and share.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Language {
    pub name: String,
    pub bytes: u64,
    pub share: f64,
}

/// Map file extensions to language names.
fn extension_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    // Rust
    map.insert("rs", "Rust");
    // JavaScript/TypeScript
    map.insert("js", "JavaScript");
    map.insert("jsx", "JavaScript");
    map.insert("ts", "TypeScript");
    map.insert("tsx", "TypeScript");
    map.insert("mjs", "JavaScript");
    map.insert("cjs", "JavaScript");
    // Python
    map.insert("py", "Python");
    map.insert("pyw", "Python");
    map.insert("pyi", "Python");
    // Go
    map.insert("go", "Go");
    // Java
    map.insert("java", "Java");
    map.insert("kt", "Kotlin");
    map.insert("scala", "Scala");
    // C/C++
    map.insert("c", "C");
    map.insert("h", "C");
    map.insert("cpp", "C++");
    map.insert("cc", "C++");
    map.insert("cxx", "C++");
    map.insert("hpp", "C++");
    map.insert("hh", "C++");
    // C#
    map.insert("cs", "C#");
    // Ruby
    map.insert("rb", "Ruby");
    // PHP
    map.insert("php", "PHP");
    // Swift
    map.insert("swift", "Swift");
    // Rust-adjacent
    map.insert("toml", "TOML");
    // Web
    map.insert("html", "HTML");
    map.insert("htm", "HTML");
    map.insert("css", "CSS");
    map.insert("scss", "SCSS");
    map.insert("sass", "Sass");
    map.insert("vue", "Vue");
    map.insert("svelte", "Svelte");
    // Shell
    map.insert("sh", "Shell");
    map.insert("bash", "Shell");
    map.insert("zsh", "Shell");
    map.insert("fish", "Shell");
    // Data/Config
    map.insert("json", "JSON");
    map.insert("yaml", "YAML");
    map.insert("yml", "YAML");
    map.insert("xml", "XML");
    map.insert("md", "Markdown");
    map.insert("markdown", "Markdown");
    map.insert("txt", "Text");
    // Lockfiles and build outputs -> Other
    map.insert("lock", "Other");
    map.insert("log", "Other");
    map.insert("out", "Other");
    // Docker
    map.insert("dockerfile", "Dockerfile");
    // SQL
    map.insert("sql", "SQL");
    // Lua
    map.insert("lua", "Lua");
    // R
    map.insert("r", "R");
    map.insert("R", "R");
    // Perl
    map.insert("pl", "Perl");
    map.insert("pm", "Perl");
    // Haskell
    map.insert("hs", "Haskell");
    // OCaml
    map.insert("ml", "OCaml");
    map.insert("mli", "OCaml");
    // F#
    map.insert("fs", "F#");
    map.insert("fsx", "F#");
    // Elm
    map.insert("elm", "Elm");
    // Dart
    map.insert("dart", "Dart");
    // Zig
    map.insert("zig", "Zig");
    // Nim
    map.insert("nim", "Nim");
    // Crystal
    map.insert("cr", "Crystal");
    // Julia
    map.insert("jl", "Julia");
    // V
    map.insert("v", "V");
    // Odin
    map.insert("odin", "Odin");
    // Assembly
    map.insert("asm", "Assembly");
    map.insert("s", "Assembly");
    // Make
    map.insert("mk", "Makefile");
    map.insert("mak", "Makefile");
    // CMake
    map.insert("cmake", "CMake");
    // Bazel
    map.insert("bzl", "Starlark");
    // Terraform
    map.insert("tf", "HCL");
    map.insert("tfvars", "HCL");
    // Nix
    map.insert("nix", "Nix");
    // Protobuf
    map.insert("proto", "Protobuf");
    // GraphQL
    map.insert("graphql", "GraphQL");
    map.insert("gql", "GraphQL");

    map
}

/// Detect languages from tracked files in a git repository.
pub fn detect_languages(repo_path: &Path, tracked_files: &[String]) -> Result<Vec<Language>> {
    let ext_map = extension_map();
    let mut lang_bytes: HashMap<String, u64> = HashMap::new();
    let mut total_bytes: u64 = 0;

    for file in tracked_files {
        let path = repo_path.join(file);
        if let Ok(metadata) = std::fs::metadata(&path) {
            let size = metadata.len();
            // Get extension
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let lang = ext_map.get(ext.as_str()).unwrap_or(&"Unknown");
            *lang_bytes.entry(lang.to_string()).or_insert(0) += size;
            total_bytes += size;
        }
    }

    // Convert to sorted vector
    let mut languages: Vec<Language> = lang_bytes
        .into_iter()
        .map(|(name, bytes)| Language {
            name,
            bytes,
            share: if total_bytes > 0 {
                bytes as f64 / total_bytes as f64
            } else {
                0.0
            },
        })
        .collect();

    // Sort by bytes descending
    languages.sort_by_key(|a| std::cmp::Reverse(a.bytes));

    Ok(languages)
}
