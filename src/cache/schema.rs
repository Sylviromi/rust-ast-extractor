use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Fn,
    Struct,
    Enum,
    Trait,
    Impl,
    Type,
    Const,
    Macro,
    Mod,
}

impl std::fmt::Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ItemKind::Fn => "fn",
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Trait => "trait",
            ItemKind::Impl => "impl",
            ItemKind::Type => "type",
            ItemKind::Const => "const",
            ItemKind::Macro => "macro",
            ItemKind::Mod => "mod",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedItem {
    pub kind: ItemKind,
    pub name: String,
    pub visibility: String,
    pub signature: String,
    pub docs: String,
    pub attributes: Vec<String>,
    pub item_hash: String,
    pub raw_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCache {
    pub file: String,
    pub file_hash: String,
    pub indexed_at: String,
    #[serde(default)]
    pub module_doc: String,
    pub items: Vec<ExtractedItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cache() -> FileCache {
        FileCache {
            file: "src/lib.rs".into(),
            file_hash: "sha256:abc".into(),
            indexed_at: "2026-04-26T00:00:00Z".into(),
            module_doc: String::new(),
            items: vec![ExtractedItem {
                kind: ItemKind::Fn,
                name: "my_fn".into(),
                visibility: "pub".into(),
                signature: "pub fn my_fn()".into(),
                docs: "Does a thing.".into(),
                attributes: vec!["#[inline]".into()],
                item_hash: "sha256:def".into(),
                raw_source: "pub fn my_fn() {}".into(),
            }],
        }
    }

    #[test]
    fn roundtrip_serialization() {
        let cache = sample_cache();
        let json = serde_json::to_string(&cache).unwrap();
        let decoded: FileCache = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.file, cache.file);
        assert_eq!(decoded.items[0].kind, ItemKind::Fn);
        assert_eq!(decoded.items[0].name, "my_fn");
    }

    #[test]
    fn item_kind_serializes_lowercase() {
        let json = serde_json::to_string(&ItemKind::Fn).unwrap();
        assert_eq!(json, r#""fn""#);
        let json = serde_json::to_string(&ItemKind::Impl).unwrap();
        assert_eq!(json, r#""impl""#);
    }
}
