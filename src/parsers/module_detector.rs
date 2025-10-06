//! Module system detection using AST visitor pattern

use crate::models::file_metadata::{ExportedSymbol, ImportedSymbol, ModuleSystem, SymbolType};
use crate::parsers::ast_parser::ModuleSystemAnalysis;
use oxc_ast::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// Visitor for detecting module system usage in a file
pub struct ModuleDetector<'a> {
    has_esm_syntax: bool,
    has_cjs_syntax: bool,
    imports: Vec<ImportedSymbol>,
    exports: Vec<ExportedSymbol>,
    circular_dependencies: HashSet<PathBuf>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> ModuleDetector<'a> {
    /// Analyze a program AST to detect module system
    pub fn analyze(program: &Program<'a>) -> ModuleSystemAnalysis {
        let mut detector = Self {
            has_esm_syntax: false,
            has_cjs_syntax: false,
            imports: Vec::new(),
            exports: Vec::new(),
            circular_dependencies: HashSet::new(),
            _phantom: std::marker::PhantomData,
        };

        // Process the program body
        detector.visit_program(program);

        ModuleSystemAnalysis {
            module_system: detector.determine_module_system(),
            imports: detector.imports,
            exports: detector.exports,
            circular_dependencies: detector.circular_dependencies,
            has_errors: false,
            parse_errors: Vec::new(),
        }
    }

    fn visit_program(&mut self, program: &Program<'a>) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::ImportDeclaration(decl) => self.visit_import_declaration(decl),
            Statement::ExportNamedDeclaration(decl) => self.visit_export_named_declaration(decl),
            Statement::ExportDefaultDeclaration(decl) => self.visit_export_default_declaration(decl),
            Statement::ExportAllDeclaration(decl) => self.visit_export_all_declaration(decl),
            Statement::ExpressionStatement(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression);
            }
            Statement::VariableDeclaration(var_decl) => {
                for declarator in &var_decl.declarations {
                    if let Some(init) = &declarator.init {
                        self.visit_expression(init);
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_expression(&mut self, expr: &Expression<'a>) {
        match expr {
            Expression::CallExpression(call_expr) => self.visit_call_expression(call_expr),
            Expression::AssignmentExpression(assign_expr) => self.visit_assignment_expression(assign_expr),
            Expression::ImportExpression(import_expr) => {
                // Handle dynamic import()
                let source = if let Expression::StringLiteral(lit) = &import_expr.source {
                    lit.value.as_str().to_string()
                } else {
                    "<dynamic>".to_string()
                };

                self.imports.push(ImportedSymbol {
                    source,
                    imported_names: vec!["<dynamic>".to_string()],
                    is_dynamic: true,
                    line_number: self.get_line_number(import_expr.span),
                });
            }
            _ => {
                // Handle MemberExpression variants through inheritance
                if let Some(member_expr) = expr.as_member_expression() {
                    self.visit_member_expression(member_expr);
                }
            }
        }
    }

    fn determine_module_system(&self) -> ModuleSystem {
        match (self.has_esm_syntax, self.has_cjs_syntax) {
            (true, true) => ModuleSystem::Mixed,
            (true, false) => ModuleSystem::ESM,
            (false, true) => ModuleSystem::CommonJS,
            (false, false) => ModuleSystem::Unknown,
        }
    }

    fn get_line_number(&self, span: oxc_span::Span) -> usize {
        // For simplicity, using byte position as line number
        // In production, would need source map for accurate line numbers
        span.start as usize
    }
}

impl<'a> ModuleDetector<'a> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        self.has_esm_syntax = true;

        let source = decl.source.value.as_str();
        let mut imported_names = Vec::new();

        // Process import specifiers
        if let Some(specifiers) = &decl.specifiers {
            for specifier in specifiers {
                match &specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                        imported_names.push(spec.imported.name().to_string());
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        imported_names.push("default".to_string());
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
                        imported_names.push(format!("* as {}", spec.local.name));
                    }
                }
            }
        }

        self.imports.push(ImportedSymbol {
            source: source.to_string(),
            imported_names,
            is_dynamic: false,
            line_number: self.get_line_number(decl.span),
        });

        self.walk_import_declaration(decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        self.has_esm_syntax = true;

        // Handle export specifiers
        for specifier in &decl.specifiers {
            let exported = &specifier.exported;
            self.exports.push(ExportedSymbol {
                name: exported.name().to_string(),
                symbol_type: SymbolType::Unknown,
                is_default: false,
                line_number: self.get_line_number(decl.span),
            });
        }

        // Handle export declarations (e.g., export const x = 1)
        if let Some(declaration) = &decl.declaration {
            match declaration {
                Declaration::VariableDeclaration(var_decl) => {
                    for declarator in &var_decl.declarations {
                        if let BindingPatternKind::BindingIdentifier(id) = &declarator.id.kind {
                            self.exports.push(ExportedSymbol {
                                name: id.name.to_string(),
                                symbol_type: SymbolType::Variable,
                                is_default: false,
                                line_number: self.get_line_number(var_decl.span),
                            });
                        }
                    }
                }
                Declaration::FunctionDeclaration(func_decl) => {
                    if let Some(id) = &func_decl.id {
                        self.exports.push(ExportedSymbol {
                            name: id.name.to_string(),
                            symbol_type: SymbolType::Function,
                            is_default: false,
                            line_number: self.get_line_number(func_decl.span),
                        });
                    }
                }
                Declaration::ClassDeclaration(class_decl) => {
                    if let Some(id) = &class_decl.id {
                        self.exports.push(ExportedSymbol {
                            name: id.name.to_string(),
                            symbol_type: SymbolType::Class,
                            is_default: false,
                            line_number: self.get_line_number(class_decl.span),
                        });
                    }
                }
                _ => {}
            }
        }

        self.walk_export_named_declaration(decl);
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        self.has_esm_syntax = true;

        self.exports.push(ExportedSymbol {
            name: "default".to_string(),
            symbol_type: match &decl.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(_) => SymbolType::Function,
                ExportDefaultDeclarationKind::ClassDeclaration(_) => SymbolType::Class,
                _ => SymbolType::Unknown,
            },
            is_default: true,
            line_number: self.get_line_number(decl.span),
        });

        self.walk_export_default_declaration(decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        self.has_esm_syntax = true;

        let source = decl.source.value.as_str();
        self.imports.push(ImportedSymbol {
            source: source.to_string(),
            imported_names: vec!["*".to_string()],
            is_dynamic: false,
            line_number: self.get_line_number(decl.span),
        });

        self.walk_export_all_declaration(decl);
    }

    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        // Check for require() calls
        if let Expression::Identifier(ident) = &expr.callee {
            if ident.name == "require" && expr.arguments.len() > 0 {
                self.has_cjs_syntax = true;

                // Extract the required module path
                if let Some(Argument::StringLiteral(lit)) = expr.arguments.first() {
                    self.imports.push(ImportedSymbol {
                        source: lit.value.as_str().to_string(),
                        imported_names: vec!["*".to_string()],
                        is_dynamic: false,
                        line_number: self.get_line_number(expr.span),
                    });
                }
            }
        }

        // Check for dynamic import() calls
        // Import expressions are handled separately in Expression enum
        // This is handled in visit_expression through Expression::ImportExpression

        self.walk_call_expression(expr);
    }

    fn visit_member_expression(&mut self, expr: &MemberExpression<'a>) {
        // Detect module.exports and exports.xxx
        // MemberExpression is an enum with ComputedMemberExpression, StaticMemberExpression, PrivateFieldExpression
        match expr {
            MemberExpression::StaticMemberExpression(static_expr) => {
                // Check if this is module.exports or exports.xxx
                if let Expression::Identifier(ident) = &static_expr.object {
                    if ident.name == "exports" {
                        self.has_cjs_syntax = true;
                    }
                }
            }
            MemberExpression::ComputedMemberExpression(computed_expr) => {
                // Handle computed member expressions if needed
                if let Expression::Identifier(ident) = &computed_expr.object {
                    if ident.name == "exports" || ident.name == "module" {
                        self.has_cjs_syntax = true;
                    }
                }
            }
            _ => {}
        }

        self.walk_member_expression(expr);
    }

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        // Check for module.exports = or exports.xxx = assignments
        // AssignmentTarget is an enum with different types
        match &expr.left {
            AssignmentTarget::StaticMemberExpression(static_expr) => {
                // Check for exports.foo = ... or module.exports = ...
                // For CommonJS detection, we just need to know it's being used
                self.has_cjs_syntax = true;
                self.exports.push(ExportedSymbol {
                    name: static_expr.property.name.to_string(),
                    symbol_type: SymbolType::Unknown,
                    is_default: false,
                    line_number: self.get_line_number(expr.span),
                });
            }
            _ => {}
        }
    }
}

// Implement walk methods (these would normally be auto-generated)
impl<'a> ModuleDetector<'a> {
    fn walk_import_declaration(&mut self, _decl: &ImportDeclaration<'a>) {
        // Continue traversal - in real implementation, would traverse child nodes
    }

    fn walk_export_named_declaration(&mut self, _decl: &ExportNamedDeclaration<'a>) {
        // Continue traversal
    }

    fn walk_export_default_declaration(&mut self, _decl: &ExportDefaultDeclaration<'a>) {
        // Continue traversal
    }

    fn walk_export_all_declaration(&mut self, _decl: &ExportAllDeclaration<'a>) {
        // Continue traversal
    }

    fn walk_call_expression(&mut self, _expr: &CallExpression<'a>) {
        // Continue traversal
    }

    fn walk_member_expression(&mut self, _expr: &MemberExpression<'a>) {
        // Continue traversal
    }

    fn walk_assignment_expression(&mut self, _expr: &AssignmentExpression<'a>) {
        // Continue traversal
    }
}