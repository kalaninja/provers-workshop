//! Codama IDL generator for the ticketing program (run via `just solana
//! gen-idl`).
//!
//! Walks this crate's source AST with codama and writes the codama-native IDL
//! to `idl/ticketing.json` at the workspace root. Built only under the
//! `idl-gen` feature (see this crate's `Cargo.toml`), so ordinary host/SBF
//! builds never compile the codama machinery.
//!
//! codama 0.9 pulls `cargo_toml` 0.20, which cannot parse `resolver = "3"`, so
//! it fails to resolve this crate's `workspace = true` inheritance against the
//! real workspace manifest. We sidestep that by pointing codama at a
//! self-contained throwaway manifest written outside the workspace tree, whose
//! `[lib]` path references the real source. codama only parses the AST — it
//! never builds — so the throwaway manifest needs no dependencies.
//!
//! ## codama post-processing fixups
//!
//! codama 0.9 omits or over-nests three things this program wants in its IDL,
//! each handled by a self-contained module (defined below `main`) that rides
//! codama's WIP korok/node API:
//!
//! - [`docs`] injects `///` doc comments (codama ignores `#[doc]`);
//! - [`skip`] applies `#[codama(skip)]` to struct fields (codama parses the
//!   directive but only honours it on enum variants);
//! - [`flatten`] inlines an instruction's single `args: XxxArgs` wrapper into
//!   the struct's own fields, so the IDL uses the conventional flat-argument
//!   shape and PDA `default_value`s can reference an arg field by name.
//!
//! [`generate_idl_json`] runs the three in order — docs, skip, flatten — one
//! line each, between codama building the node tree and serializing it. Comment
//! out a line to drop that fixup. Order is load-bearing only in that `flatten`
//! carries over the field docs `docs` injects, and inlines post-`skip` fields.
//! A codama upgrade may require adjusting these modules.

use std::{env, fs, path::Path, process};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This bin is part of the interface crate, so its compile-time crate dir and
    // version match the interface's (workspace inheritance already resolved),
    // and `package_metadata!` reads the same `program-id` `declare_id!` uses.
    let crate_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let program_id = solana_package_metadata::package_metadata!("solana.program-id");

    // Self-contained scratch crate, placed OUTSIDE the workspace tree so codama's
    // cargo_toml does not walk up into the real `resolver = "3"` workspace. Keyed
    // by process id so concurrent runs (or another user's leftover directory on
    // a shared host) cannot collide.
    let scratch = env::temp_dir().join(format!("codama-ticketing-idl-{}", process::id()));
    fs::create_dir_all(&scratch)?;
    let lib_path = crate_path.join("src/lib.rs");
    let version = env!("CARGO_PKG_VERSION");
    let scratch_manifest = format!(
        "[package]\n\
         name = \"ticketing-interface\"\n\
         version = \"{version}\"\n\
         edition = \"2021\"\n\n\
         [lib]\n\
         path = \"{lib}\"\n\n\
         [package.metadata.solana]\n\
         program-id = \"{program_id}\"\n",
        lib = lib_path.display(),
    );
    fs::write(scratch.join("Cargo.toml"), scratch_manifest)?;

    let codama = codama::Codama::load(&scratch)?;
    let idl_json = generate_idl_json(&codama)?;

    // Normalise the program name (codama derives it from the crate name, which
    // carries the `-interface` role suffix) and pretty-print.
    let mut idl: serde_json::Value = serde_json::from_str(&idl_json)?;
    if let Some(program) = idl.get_mut("program").and_then(|p| p.as_object_mut()) {
        program.insert("name".to_string(), serde_json::json!("ticketing"));
        size_account_discriminators(program);
    }
    let mut pretty = serde_json::to_string_pretty(&idl)?;
    pretty.push('\n');

    let out_path = crate_path.join("../../../idl/ticketing.json");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out_path, pretty)?;
    println!("codama IDL written to {}", out_path.display());

    // Best-effort cleanup; a leftover scratch dir only wastes bytes in tmp.
    let _ = fs::remove_dir_all(&scratch);

    Ok(())
}

/// codama 0.9's `#[codama(discriminator(bytes = ...))]` records the constant as
/// an unbounded `bytesTypeNode`, so a renderer cannot tell the discriminator is
/// a fixed-width array. The byte length is known from the literal value, so
/// wrap each account discriminator constant in a `fixedSizeTypeNode` — the same
/// `fixedSize(bytes, N)` those bytes already carry as the `header` field, which
/// keeps the two views of the discriminator consistent and lets clients render
/// it as `[u8; N]`. Bare-bytes only; `field`/`size` discriminators carry no
/// inline constant and are left untouched.
fn size_account_discriminators(program: &mut serde_json::Map<String, serde_json::Value>) {
    let Some(accounts) = program.get_mut("accounts").and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    for account in accounts {
        let Some(discriminators) =
            account.get_mut("discriminators").and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        for discriminator in discriminators {
            let Some(constant) =
                discriminator.get_mut("constant").and_then(serde_json::Value::as_object_mut)
            else {
                continue;
            };
            let is_bare_bytes = constant
                .get("type")
                .and_then(|node| node.get("kind"))
                .and_then(serde_json::Value::as_str)
                == Some("bytesTypeNode");
            let Some(size) = base16_byte_len(constant.get("value")) else {
                continue;
            };
            if !is_bare_bytes {
                continue;
            }
            let Some(inner) = constant.remove("type") else {
                continue;
            };
            constant.insert(
                "type".to_string(),
                serde_json::json!({ "kind": "fixedSizeTypeNode", "size": size, "type": inner }),
            );
        }
    }
}

/// Byte length of a base16 `bytesValueNode`, or `None` for other encodings.
fn base16_byte_len(value: Option<&serde_json::Value>) -> Option<usize> {
    let value = value?;
    if value.get("encoding").and_then(serde_json::Value::as_str)? != "base16" {
        return None;
    }
    Some(value.get("data").and_then(serde_json::Value::as_str)?.len() / 2)
}

/// Builds the ticketing IDL JSON, applying the doc, skip, and flatten fixups
/// codama 0.9 omits. To emit codama's native IDL, replace the body with
/// `codama.get_json_idl()`; to drop one fixup, comment out its line.
fn generate_idl_json(codama: &codama::Codama) -> Result<String, Box<dyn std::error::Error>> {
    use codama::{Node, NodeTrait};

    let mut korok = codama.get_visited_korok()?;

    docs::inject(&mut korok)?; // comment out to emit doc-less nodes
    skip::apply(&mut korok)?; // comment out to keep #[codama(skip)] fields
    flatten::apply(&mut korok)?; // comment out to keep nested `args` wrappers

    let Some(Node::Root(root)) = korok.node.as_ref() else {
        return Err("codama produced no root node".into());
    };
    Ok(root.to_json()?)
}

/// `///` doc-comment injection: codama 0.9 does not read `#[doc]` comments into
/// the IDL, so this walks the source AST and copies the summary line of each
/// account, defined-type, instruction, and struct-field doc onto the matching
/// node. This is `syn`'s only user — drop the `syn` dependency if this module
/// is removed.
mod docs {
    use std::collections::HashMap;

    use codama::{KorokVisitor, Node};

    /// Copies `///` docs from the source AST onto the program's account,
    /// defined-type, and instruction nodes (and their struct fields).
    pub(crate) fn inject(korok: &mut codama::RootKorok) -> codama::CodamaResult<()> {
        let mut collector = Collector::default();
        collector.visit_root(korok)?;
        if let Some(Node::Root(root)) = korok.node.as_mut() {
            collector.apply(&mut root.program);
        }
        Ok(())
    }

    /// Collected docs, keyed by each item's codama (camelCase) name — the same
    /// casing the nodes use, so [`Collector::apply`] looks them up directly.
    #[derive(Default)]
    struct Collector {
        /// codama item name -> doc lines (accounts, defined types,
        /// instructions).
        items: HashMap<String, codama::Docs>,
        /// struct item name -> (field name -> doc lines).
        fields: HashMap<String, HashMap<String, codama::Docs>>,
    }

    impl Collector {
        fn apply(&self, program: &mut codama::ProgramNode) {
            for account in &mut program.accounts {
                // Owned, because the field-doc pass below borrows `account.data`
                // mutably while we still need the name.
                let name = account.name.to_string();
                if account.docs.is_empty()
                    && let Some(docs) = self.items.get(&name)
                {
                    account.docs = docs.clone();
                }
                // Account data is an unwrapped struct (no size/offset wrappers),
                // so its fields sit directly under `NestedTypeNode::Value`.
                if let codama::NestedTypeNode::Value(struct_node) = &mut account.data {
                    self.apply_fields(&name, struct_node);
                }
            }
            for defined_type in &mut program.defined_types {
                let name = defined_type.name.to_string();
                if defined_type.docs.is_empty()
                    && let Some(docs) = self.items.get(&name)
                {
                    defined_type.docs = docs.clone();
                }
                // Enum variant nodes carry no docs field in codama 0.9, so only
                // struct defined types can take field docs.
                if let codama::TypeNode::Struct(struct_node) = &mut defined_type.r#type {
                    self.apply_fields(&name, struct_node);
                }
            }
            for instruction in &mut program.instructions {
                if instruction.docs.is_empty()
                    && let Some(docs) = self.items.get(instruction.name.as_str())
                {
                    instruction.docs = docs.clone();
                }
            }
        }

        fn apply_fields(&self, parent: &str, struct_node: &mut codama::StructTypeNode) {
            let Some(field_docs) = self.fields.get(parent) else {
                return;
            };
            for field in &mut struct_node.fields {
                if field.docs.is_empty()
                    && let Some(docs) = field_docs.get(field.name.as_str())
                {
                    field.docs = docs.clone();
                }
            }
        }
    }

    impl KorokVisitor for Collector {
        fn visit_struct(&mut self, korok: &mut codama::StructKorok) -> codama::CodamaResult<()> {
            let name = korok.name().to_string();
            if let Some(docs) = read_docs(&korok.ast.attrs) {
                self.items.insert(name.clone(), docs);
            }
            let field_docs: HashMap<String, codama::Docs> = korok
                .fields
                .iter()
                .filter_map(|field| Some((field.name()?.to_string(), read_docs(&field.ast.attrs)?)))
                .collect();
            if !field_docs.is_empty() {
                self.fields.insert(name, field_docs);
            }
            Ok(())
        }

        fn visit_enum(&mut self, korok: &mut codama::EnumKorok) -> codama::CodamaResult<()> {
            // The instruction enum contributes one node per variant; every other
            // enum (status types) is a single defined type.
            if korok.attributes.has_codama_derive("CodamaInstructions") {
                for variant in &korok.variants {
                    if let Some(docs) = read_docs(&variant.ast.attrs) {
                        self.items.insert(variant.name().to_string(), docs);
                    }
                }
            } else if let Some(docs) = read_docs(&korok.ast.attrs) {
                self.items.insert(korok.name().to_string(), docs);
            }
            Ok(())
        }
    }

    /// Extracts the summary paragraph from `#[doc = "..."]` attributes (i.e.
    /// `///` comments), trimming the single leading space rustdoc inserts. Only
    /// the first paragraph is kept: later paragraphs hold internal notes
    /// (memory-layout tables, schema-evolution remarks) that don't belong in a
    /// client-facing IDL. Returns `None` when the item carries no doc comment.
    fn read_docs(attrs: &[syn::Attribute]) -> Option<codama::Docs> {
        let lines: Vec<String> = attrs
            .iter()
            .filter_map(|attr| {
                if !attr.path().is_ident("doc") {
                    return None;
                }
                let syn::Meta::NameValue(name_value) = &attr.meta else {
                    return None;
                };
                let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(text), .. }) =
                    &name_value.value
                else {
                    return None;
                };
                Some(text.value().trim().to_string())
            })
            .take_while(|line| !line.is_empty())
            .collect();

        (!lines.is_empty()).then(|| lines.into())
    }
}

/// `#[codama(skip)]` field removal: codama 0.9 parses the directive into a
/// `SkipDirective` but only honours it on enum variants, so a `skip` on a
/// struct field is silently ignored. This walks the source AST for fields
/// marked `#[codama(skip)]` and drops them from the matching account /
/// defined-type nodes. Enum-variant skip is left to codama; instruction
/// *arguments* are out of scope (codama flattens them into a separate node
/// path).
///
/// Skipping a field that carries bytes shrinks the IDL below the runtime
/// `LEN`, so this is meant for zero-size / non-wire fields.
mod skip {
    use std::collections::{HashMap, HashSet};

    use codama::{KorokVisitor, Node};

    /// Drops `#[codama(skip)]` struct fields from the program's account and
    /// defined-type nodes.
    pub(crate) fn apply(korok: &mut codama::RootKorok) -> codama::CodamaResult<()> {
        let mut collector = Collector::default();
        collector.visit_root(korok)?;
        if let Some(Node::Root(root)) = korok.node.as_mut() {
            collector.apply(&mut root.program);
        }
        Ok(())
    }

    #[derive(Default)]
    struct Collector {
        /// struct item name -> field names carrying `#[codama(skip)]`.
        skips: HashMap<String, HashSet<String>>,
    }

    impl Collector {
        fn apply(&self, program: &mut codama::ProgramNode) {
            for account in &mut program.accounts {
                let name = account.name.to_string();
                if let codama::NestedTypeNode::Value(struct_node) = &mut account.data {
                    self.retain(&name, struct_node);
                }
            }
            for defined_type in &mut program.defined_types {
                let name = defined_type.name.to_string();
                if let codama::TypeNode::Struct(struct_node) = &mut defined_type.r#type {
                    self.retain(&name, struct_node);
                }
            }
        }

        fn retain(&self, parent: &str, struct_node: &mut codama::StructTypeNode) {
            if let Some(skipped) = self.skips.get(parent) {
                struct_node.fields.retain(|field| !skipped.contains(field.name.as_str()));
            }
        }
    }

    impl KorokVisitor for Collector {
        fn visit_struct(&mut self, korok: &mut codama::StructKorok) -> codama::CodamaResult<()> {
            // Reuse codama's own attribute lookup — the same call it makes for
            // enum-variant skip.
            let skipped: HashSet<String> = korok
                .fields
                .iter()
                .filter(|field| field.attributes.has_codama_attribute("skip"))
                .filter_map(|field| Some(field.name()?.to_string()))
                .collect();
            if !skipped.is_empty() {
                self.skips.insert(korok.name().to_string(), skipped);
            }
            Ok(())
        }
    }
}

/// `args`-wrapper flattening: an instruction whose Rust variant carries its
/// payload in a single `args: XxxArgs` field renders as one struct-typed
/// argument (a `definedTypeLink`). codama keeps that nesting, but the
/// conventional IDL shape is a flat argument list — and, more importantly, a
/// PDA `default_value` seed can only reference a *top-level* argument by name,
/// not a field inside a struct argument. This inlines each such wrapper —
/// replacing the link argument with the struct's fields as individual
/// arguments (codama's `From<StructTypeNode>` carries names, types, and docs)
/// — and drops the now-orphaned wrapper defined types. Pure node-tree edit; no
/// source-AST pass. The discriminator argument is a `number` type, not a link,
/// so it is left untouched; nested field types (e.g. `eventName`) keep their
/// links and defined types.
mod flatten {
    use std::collections::{HashMap, HashSet};

    use codama::{InstructionArgumentNode, Node, StructTypeNode, TypeNode};

    /// Inlines every single struct-link instruction argument into the linked
    /// struct's fields, then prunes the orphaned wrapper defined types.
    ///
    /// Infallible, but mirrors the `-> CodamaResult<()>` shape of `docs`/`skip`
    /// so `generate_idl_json` drives all three the same way.
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn apply(korok: &mut codama::RootKorok) -> codama::CodamaResult<()> {
        let Some(Node::Root(root)) = korok.node.as_mut() else {
            return Ok(());
        };
        let program = &mut root.program;

        // Index every struct defined type by its (camelCase) node name so a
        // `definedTypeLink` argument can be resolved to its fields.
        let structs: HashMap<String, StructTypeNode> = program
            .defined_types
            .iter()
            .filter_map(|defined_type| match &defined_type.r#type {
                TypeNode::Struct(node) => Some((defined_type.name.to_string(), node.clone())),
                _ => None,
            })
            .collect();

        let mut inlined = HashSet::new();
        for instruction in &mut program.instructions {
            let mut flattened = Vec::with_capacity(instruction.arguments.len());
            for argument in std::mem::take(&mut instruction.arguments) {
                match &argument.r#type {
                    TypeNode::Link(link) if structs.contains_key(link.name.as_str()) => {
                        let name = link.name.to_string();
                        flattened
                            .extend(Vec::<InstructionArgumentNode>::from(structs[&name].clone()));
                        inlined.insert(name);
                    },
                    _ => flattened.push(argument),
                }
            }
            instruction.arguments = flattened;
        }

        // Each wrapper struct backs exactly one instruction and is referenced
        // nowhere else, so the inlined ones are now unreferenced.
        program.defined_types.retain(|defined_type| !inlined.contains(defined_type.name.as_str()));
        Ok(())
    }
}
