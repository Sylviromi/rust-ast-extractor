use crate::cache::schema::{ExtractedItem, ItemKind};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct ItemVisitor<'src> {
    pub items: Vec<ExtractedItem>,
    source: &'src str,
}

impl<'src> ItemVisitor<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            items: Vec::new(),
            source,
        }
    }
}

// — helpers —

fn extract_docs(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
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

fn extract_non_doc_attrs(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| !a.path().is_ident("doc"))
        .map(|a| a.to_token_stream().to_string())
        .collect()
}

fn visibility_str(vis: &syn::Visibility) -> String {
    vis.to_token_stream().to_string()
}

fn extract_lines(source: &str, start_line: usize, end_line: usize) -> String {
    source
        .lines()
        .enumerate()
        .filter(|(i, _)| *i >= start_line.saturating_sub(1) && *i < end_line)
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn item_hash(tokens: proc_macro2::TokenStream) -> String {
    crate::cache::compute_hash(&tokens.to_string())
}

// — visitor impl —

impl<'src, 'ast> Visit<'ast> for ItemVisitor<'src> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let span = i.span();
        let start = span.start().line;
        let end = span.end().line;
        let raw = extract_lines(self.source, start, end);
        let vis = visibility_str(&i.vis);
        let sig = format!("{} {}", vis, i.sig.to_token_stream()).trim().to_string();

        self.items.push(ExtractedItem {
            kind: ItemKind::Fn,
            name: i.sig.ident.to_string(),
            visibility: vis,
            signature: sig,
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: raw,
        });
        // do NOT recurse — no nested fn extraction
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);
        let vis = visibility_str(&i.vis);
        self.items.push(ExtractedItem {
            kind: ItemKind::Struct,
            name: i.ident.to_string(),
            visibility: vis.clone(),
            signature: format!("{} struct {}", vis, i.ident).trim().to_string(),
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });
    }

    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);
        let vis = visibility_str(&i.vis);
        self.items.push(ExtractedItem {
            kind: ItemKind::Enum,
            name: i.ident.to_string(),
            visibility: vis.clone(),
            signature: format!("{} enum {}", vis, i.ident).trim().to_string(),
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });
    }

    fn visit_item_trait(&mut self, i: &'ast syn::ItemTrait) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);
        let vis = visibility_str(&i.vis);
        self.items.push(ExtractedItem {
            kind: ItemKind::Trait,
            name: i.ident.to_string(),
            visibility: vis.clone(),
            signature: format!("{} trait {}", vis, i.ident).trim().to_string(),
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });
        // do NOT recurse — trait methods not extracted separately
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);

        let name = if let Some((_, trait_path, _)) = &i.trait_ {
            format!(
                "{} for {}",
                trait_path.to_token_stream(),
                i.self_ty.to_token_stream()
            )
        } else {
            i.self_ty.to_token_stream().to_string()
        };

        let sig = format!("impl {name}");

        self.items.push(ExtractedItem {
            kind: ItemKind::Impl,
            name: name.clone(),
            visibility: String::new(),
            signature: sig,
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });

        // recurse into impl to collect methods as Fn items
        for item in &i.items {
            if let syn::ImplItem::Fn(method) = item {
                let mspan = method.span();
                let (ms, me) = (mspan.start().line, mspan.end().line);
                let vis = visibility_str(&method.vis);
                let msig = format!("{} {}", vis, method.sig.to_token_stream()).trim().to_string();
                self.items.push(ExtractedItem {
                    kind: ItemKind::Fn,
                    name: method.sig.ident.to_string(),
                    visibility: vis,
                    signature: msig,
                    docs: extract_docs(&method.attrs),
                    attributes: extract_non_doc_attrs(&method.attrs),
                    item_hash: item_hash(method.to_token_stream()),
                    raw_source: extract_lines(self.source, ms, me),
                });
            }
        }
    }

    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);
        let vis = visibility_str(&i.vis);
        self.items.push(ExtractedItem {
            kind: ItemKind::Type,
            name: i.ident.to_string(),
            visibility: vis.clone(),
            signature: format!("{} type {}", vis, i.ident).trim().to_string(),
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);
        let vis = visibility_str(&i.vis);
        self.items.push(ExtractedItem {
            kind: ItemKind::Const,
            name: i.ident.to_string(),
            visibility: vis.clone(),
            signature: format!("{} const {}: {}", vis, i.ident, i.ty.to_token_stream())
                .trim()
                .to_string(),
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });
    }

    fn visit_item_macro(&mut self, i: &'ast syn::ItemMacro) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);
        let name = i
            .ident
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_else(|| i.mac.path.to_token_stream().to_string());
        self.items.push(ExtractedItem {
            kind: ItemKind::Macro,
            name,
            visibility: String::new(),
            signature: i.mac.path.to_token_stream().to_string(),
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        let span = i.span();
        let (start, end) = (span.start().line, span.end().line);
        let vis = visibility_str(&i.vis);
        self.items.push(ExtractedItem {
            kind: ItemKind::Mod,
            name: i.ident.to_string(),
            visibility: vis.clone(),
            signature: format!("{} mod {}", vis, i.ident).trim().to_string(),
            docs: extract_docs(&i.attrs),
            attributes: extract_non_doc_attrs(&i.attrs),
            item_hash: item_hash(i.to_token_stream()),
            raw_source: extract_lines(self.source, start, end),
        });
        // recurse into inline mod bodies
        syn::visit::visit_item_mod(self, i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::visit::Visit;

    fn collect(src: &str) -> Vec<ExtractedItem> {
        let file = syn::parse_str::<syn::File>(src).expect("parse failed");
        let mut visitor = ItemVisitor::new(src);
        visitor.visit_file(&file);
        visitor.items
    }

    #[test]
    fn extracts_public_fn() {
        let src = r#"/// Does a thing.
pub fn hello(x: u32) -> String {
    x.to_string()
}"#;
        let items = collect(src);
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.kind, ItemKind::Fn);
        assert_eq!(item.name, "hello");
        assert_eq!(item.visibility, "pub");
        assert!(item.signature.contains("pub fn hello"), "got: {}", item.signature);
        assert_eq!(item.docs, "Does a thing.");
        assert!(item.raw_source.contains("pub fn hello"));
        assert!(!item.item_hash.is_empty());
    }

    #[test]
    fn extracts_struct() {
        let src = "pub struct Foo { pub x: u32 }";
        let items = collect(src);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Struct);
        assert_eq!(items[0].name, "Foo");
    }

    #[test]
    fn extracts_enum() {
        let src = "pub enum Color { Red, Green, Blue }";
        let items = collect(src);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Enum);
        assert_eq!(items[0].name, "Color");
    }

    #[test]
    fn extracts_trait() {
        let src = "pub trait Animal { fn speak(&self); }";
        let items = collect(src);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Trait);
        assert_eq!(items[0].name, "Animal");
    }

    #[test]
    fn extracts_impl_and_methods() {
        let src = r#"
struct Dog;
impl Dog {
    pub fn bark(&self) {}
}
"#;
        let items = collect(src);
        let kinds: Vec<_> = items.iter().map(|i| (&i.kind, i.name.as_str())).collect();
        assert!(kinds.contains(&(&ItemKind::Struct, "Dog")));
        assert!(kinds.contains(&(&ItemKind::Impl, "Dog")));
        assert!(kinds.contains(&(&ItemKind::Fn, "bark")));
    }

    #[test]
    fn extracts_type_alias() {
        let src = "pub type Result<T> = std::result::Result<T, String>;";
        let items = collect(src);
        assert_eq!(items[0].kind, ItemKind::Type);
        assert_eq!(items[0].name, "Result");
    }

    #[test]
    fn extracts_const() {
        let src = "pub const MAX: u32 = 100;";
        let items = collect(src);
        assert_eq!(items[0].kind, ItemKind::Const);
        assert_eq!(items[0].name, "MAX");
    }

    #[test]
    fn extracts_mod() {
        let src = "pub mod utils {}";
        let items = collect(src);
        assert!(items.iter().any(|i| i.kind == ItemKind::Mod && i.name == "utils"));
    }

    #[test]
    fn separates_doc_from_other_attrs() {
        let src = r#"
#[inline]
#[allow(dead_code)]
/// My function.
pub fn foo() {}
"#;
        let items = collect(src);
        assert_eq!(items[0].docs, "My function.");
        assert_eq!(items[0].attributes.len(), 2);
        assert!(items[0].attributes.iter().any(|a| a.contains("inline")));
    }
}
