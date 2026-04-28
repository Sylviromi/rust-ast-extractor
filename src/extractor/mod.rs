pub mod visitor;

use crate::cache::schema::ExtractedItem;
use std::path::Path;

pub struct ExtractedFile {
    pub items: Vec<ExtractedItem>,
    pub module_doc: String,
}

pub fn extract_file(_path: &Path, source: &str) -> Result<ExtractedFile, syn::Error> {
    let file = syn::parse_str::<syn::File>(source)?;
    let module_doc = extract_inner_docs(&file.attrs);
    let mut visitor = visitor::ItemVisitor::new(source);
    syn::visit::Visit::visit_file(&mut visitor, &file);
    Ok(ExtractedFile {
        items: visitor.items,
        module_doc,
    })
}

fn extract_inner_docs(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if !matches!(attr.style, syn::AttrStyle::Inner(_)) {
                return None;
            }
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let syn::Meta::NameValue(nv) = &attr.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
            {
                return Some(s.value().trim().to_string());
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::schema::ItemKind;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn extracts_items_from_file_content() {
        let src = r#"
/// A greeting.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}")
}

pub struct Config {
    pub debug: bool,
}
"#;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("lib.rs");
        fs::write(&path, src).unwrap();

        let extracted = extract_file(&path, src).unwrap();
        assert!(
            extracted
                .items
                .iter()
                .any(|i| i.kind == ItemKind::Fn && i.name == "greet")
        );
        assert!(
            extracted
                .items
                .iter()
                .any(|i| i.kind == ItemKind::Struct && i.name == "Config")
        );
    }

    #[test]
    fn extracts_module_doc() {
        let src = "//! Top-level module.\n//! Second line.\n\npub fn foo() {}";
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("lib.rs");
        fs::write(&path, src).unwrap();
        let extracted = extract_file(&path, src).unwrap();
        assert_eq!(extracted.module_doc, "Top-level module.\nSecond line.");
    }

    #[test]
    fn module_doc_empty_when_absent() {
        let src = "pub fn foo() {}";
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("lib.rs");
        fs::write(&path, src).unwrap();
        let extracted = extract_file(&path, src).unwrap();
        assert_eq!(extracted.module_doc, "");
    }

    #[test]
    fn returns_error_on_invalid_rust() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.rs");
        let bad = "fn broken( {";
        fs::write(&path, bad).unwrap();
        assert!(extract_file(&path, bad).is_err());
    }
}
