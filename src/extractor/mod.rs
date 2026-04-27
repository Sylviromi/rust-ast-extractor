pub mod visitor;

use crate::cache::schema::ExtractedItem;
use std::path::Path;

pub fn extract_file(_path: &Path, source: &str) -> Result<Vec<ExtractedItem>, syn::Error> {
    let file = syn::parse_str::<syn::File>(source)?;
    let mut visitor = visitor::ItemVisitor::new(source);
    syn::visit::Visit::visit_file(&mut visitor, &file);
    Ok(visitor.items)
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

        let items = extract_file(&path, src).unwrap();
        assert!(
            items
                .iter()
                .any(|i| i.kind == ItemKind::Fn && i.name == "greet")
        );
        assert!(
            items
                .iter()
                .any(|i| i.kind == ItemKind::Struct && i.name == "Config")
        );
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
