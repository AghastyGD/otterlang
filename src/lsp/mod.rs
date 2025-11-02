//! Language Server Protocol (LSP) implementation for OtterLang
//!
//! Provides code completion, diagnostics, hover information, and more for IDE integration

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tower_lsp::{jsonrpc::Result, lsp_types::*, LanguageServer, LspService};

use crate::lexer::{tokenize, LexerError};
use crate::module::ModuleProcessor;
use crate::parser::{parse, ParserError};
use crate::typecheck::{TypeChecker, TypeError};
use crate::utils::errors::Diagnostic;

/// LSP server state
pub struct OtterLangServer {
    workspace_root: Option<PathBuf>,
    documents: HashMap<PathBuf, String>,
    diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,
}

impl OtterLangServer {
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            documents: HashMap::new(),
            diagnostics: HashMap::new(),
        }
    }

    fn update_diagnostics(&mut self, uri: &Url) {
        if let Some(path) = self.uri_to_path(uri) {
            if let Some(content) = self.documents.get(&path) {
                let diagnostics = self.analyze_document(&path, content);
                self.diagnostics.insert(path, diagnostics);
            }
        }
    }

    fn analyze_document(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Lexical analysis
        match tokenize(content) {
            Ok(tokens) => {
                // Parse
                match parse(&tokens) {
                    Ok(program) => {
                        // Type check
                        let mut checker = TypeChecker::new();
                        if let Err(errors) = checker.check_program(&program) {
                            for error in errors {
                                diagnostics.push(Diagnostic {
                                    message: error.message,
                                    hint: error.hint,
                                    help: error.help,
                                    file: path.to_path_buf(),
                                    line: error.line,
                                    column: error.column,
                                });
                            }
                        }
                    }
                    Err(err) => {
                        diagnostics.push(Diagnostic {
                            message: format!("Parse error: {}", err),
                            hint: None,
                            help: None,
                            file: path.to_path_buf(),
                            line: 0,
                            column: 0,
                        });
                    }
                }
            }
            Err(err) => {
                diagnostics.push(Diagnostic {
                    message: format!("Lexical error: {}", err),
                    hint: None,
                    help: None,
                    file: path.to_path_buf(),
                    line: 0,
                    column: 0,
                });
            }
        }

        diagnostics
    }

    fn uri_to_path(&self, uri: &Url) -> Option<PathBuf> {
        uri.to_file_path().ok()
    }

    fn path_to_uri(&self, path: &Path) -> Option<Url> {
        Url::from_file_path(path).ok()
    }

    fn diagnostic_to_lsp(&self, diag: &Diagnostic) -> lsp_types::Diagnostic {
        let severity = lsp_types::DiagnosticSeverity::ERROR;
        let message = if let Some(ref hint) = diag.hint {
            format!("{}\nHint: {}", diag.message, hint)
        } else {
            diag.message.clone()
        };

        lsp_types::Diagnostic {
            range: Range {
                start: Position {
                    line: diag.line.saturating_sub(1) as u32,
                    character: diag.column.saturating_sub(1) as u32,
                },
                end: Position {
                    line: diag.line.saturating_sub(1) as u32,
                    character: diag.column.saturating_sub(1) as u32,
                },
            },
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("otterlang".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for OtterLangServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "otterlang-lsp".to_string(),
                version: Some(crate::version::VERSION.to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        // Server is ready
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        // Handle workspace folder changes
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        // Handle configuration changes
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        // Handle file changes
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        // Handle document open
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Handle document changes
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Handle document save
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Handle document close
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // Provide hover information
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // Provide code completion
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem {
                label: "print".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("fn print(message: string) -> unit".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "len".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("fn len(item: array | string) -> int".to_string()),
                ..Default::default()
            },
        ])))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        // Provide go-to-definition
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        // Provide references
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<Vec<DocumentSymbol>>> {
        // Provide document symbols
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = OtterLangServer::new();
        assert!(server.documents.is_empty());
    }
}

