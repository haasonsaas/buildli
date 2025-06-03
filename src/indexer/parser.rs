use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tree_sitter::{Language, Parser, Query, QueryCursor};

#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub file_path: String,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub chunk_type: ChunkType,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkType {
    Function,
    Class,
    Method,
    Module,
    Comment,
    Other,
}

static LANGUAGE_MAP: Lazy<HashMap<&str, Language>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("rs", tree_sitter_rust::language());
    map.insert("py", tree_sitter_python::language());
    map.insert("js", tree_sitter_javascript::language());
    map.insert("ts", tree_sitter_typescript::language_typescript());
    map.insert("tsx", tree_sitter_typescript::language_tsx());
    map.insert("go", tree_sitter_go::language());
    map.insert("java", tree_sitter_java::language());
    map.insert("cpp", tree_sitter_cpp::language());
    map.insert("cc", tree_sitter_cpp::language());
    map.insert("cxx", tree_sitter_cpp::language());
    map.insert("c", tree_sitter_cpp::language());
    map.insert("h", tree_sitter_cpp::language());
    map.insert("hpp", tree_sitter_cpp::language());
    map
});

pub struct LanguageParser {
    parsers: HashMap<String, Parser>,
    queries: HashMap<String, Query>,
}

impl LanguageParser {
    pub fn new() -> Self {
        let mut parsers = HashMap::new();
        let mut queries = HashMap::new();
        
        for (ext, lang) in LANGUAGE_MAP.iter() {
            let mut parser = Parser::new();
            parser.set_language(lang).unwrap();
            parsers.insert(ext.to_string(), parser);
            
            if let Ok(query) = Self::create_query_for_language(ext, lang) {
                queries.insert(ext.to_string(), query);
            }
        }
        
        Self { parsers, queries }
    }

    pub async fn parse_file(&mut self, path: &Path) -> Result<Vec<CodeChunk>> {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read file")?;
        
        let language = self.detect_language(path, &content);
        
        if self.parsers.contains_key(&language) {
            let parser = self.parsers.get_mut(&language).unwrap();
            Self::parse_with_tree_sitter(path, &content, &language, parser, &self.queries)
        } else {
            Ok(Self::fallback_parse(path, &content, &language))
        }
    }

    fn detect_language(&self, path: &Path, content: &str) -> String {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if LANGUAGE_MAP.contains_key(ext) {
                return ext.to_string();
            }
        }
        
        if content.starts_with("#!/usr/bin/env python") || content.starts_with("#!/usr/bin/python") {
            return "py".to_string();
        }
        
        if content.starts_with("#!/usr/bin/env node") || content.starts_with("#!/usr/bin/node") {
            return "js".to_string();
        }
        
        "unknown".to_string()
    }

    fn parse_with_tree_sitter(
        path: &Path,
        content: &str,
        language: &str,
        parser: &mut Parser,
        queries: &HashMap<String, Query>,
    ) -> Result<Vec<CodeChunk>> {
        let tree = parser
            .parse(content, None)
            .context("Failed to parse file")?;
        
        let mut chunks = Vec::new();
        let mut cursor = QueryCursor::new();
        
        if let Some(query) = queries.get(language) {
            let matches = cursor.matches(query, tree.root_node(), content.as_bytes());
            
            for match_ in matches {
                for capture in match_.captures {
                    let node = capture.node;
                    let start_byte = node.start_byte();
                    let end_byte = node.end_byte();
                    let chunk_content = &content[start_byte..end_byte];
                    
                    let start_line = content[..start_byte].lines().count();
                    let end_line = start_line + chunk_content.lines().count();
                    
                    let context_content = Self::create_chunk_context(content, chunk_content, start_line);
                    chunks.push(CodeChunk {
                        file_path: path.display().to_string(),
                        content: context_content,
                        line_start: start_line + 1,
                        line_end: end_line,
                        chunk_type: Self::node_to_chunk_type(&capture.node.kind()),
                        language: language.to_string(),
                    });
                }
            }
        }
        
        if chunks.is_empty() {
            chunks = Self::fallback_parse(path, content, language);
        }
        
        Ok(chunks)
    }

    fn create_query_for_language(ext: &str, language: &Language) -> Result<Query> {
        let query_string = match ext {
            "rs" => r#"
                (function_item) @function
                (impl_item) @impl
                (struct_item) @struct
                (enum_item) @enum
                (trait_item) @trait
                (mod_item) @module
            "#,
            "py" => r#"
                (function_definition) @function
                (class_definition) @class
                (decorated_definition) @decorated
            "#,
            "js" | "ts" | "tsx" => r#"
                (function_declaration) @function
                (arrow_function) @arrow_function
                (class_declaration) @class
                (method_definition) @method
            "#,
            "go" => r#"
                (function_declaration) @function
                (method_declaration) @method
                (type_declaration) @type
            "#,
            "java" => r#"
                (method_declaration) @method
                (class_declaration) @class
                (interface_declaration) @interface
            "#,
            _ => r#"
                (function_definition) @function
                (class_definition) @class
            "#,
        };
        
        Query::new(language, query_string).context("Failed to create query")
    }

    fn node_to_chunk_type(kind: &str) -> ChunkType {
        match kind {
            "function_item" | "function_declaration" | "function_definition" | "arrow_function" => {
                ChunkType::Function
            }
            "class_declaration" | "class_definition" | "struct_item" | "enum_item" => {
                ChunkType::Class
            }
            "method_declaration" | "method_definition" => ChunkType::Method,
            "mod_item" | "module" => ChunkType::Module,
            "comment" => ChunkType::Comment,
            _ => ChunkType::Other,
        }
    }

    fn create_chunk_context(full_content: &str, chunk_content: &str, start_line: usize) -> String {
        let context_lines = 3;
        let lines: Vec<&str> = full_content.lines().collect();
        
        let start = start_line.saturating_sub(context_lines);
        let chunk_lines = chunk_content.lines().count();
        let end = (start_line + chunk_lines + context_lines).min(lines.len());
        
        lines[start..end].join("\n")
    }

    fn fallback_parse(path: &Path, content: &str, language: &str) -> Vec<CodeChunk> {
        let lines: Vec<&str> = content.lines().collect();
        let chunk_size = 50;
        let overlap = 10;
        
        let mut chunks = Vec::new();
        let mut i = 0;
        
        while i < lines.len() {
            let end = (i + chunk_size).min(lines.len());
            let chunk_content = lines[i..end].join("\n");
            
            chunks.push(CodeChunk {
                file_path: path.display().to_string(),
                content: chunk_content,
                line_start: i + 1,
                line_end: end,
                chunk_type: ChunkType::Other,
                language: language.to_string(),
            });
            
            i += chunk_size - overlap;
        }
        
        chunks
    }
}