use serde::{Deserialize, Serialize};
use swc_core::ecma::visit::{AstParentKind, VisitMut};
use turbo_tasks::{debug::ValueDebugFormat, trace::TraceRawVcs, Value};
use turbopack_core::chunk::availability_info::AvailabilityInfo;

use crate::chunk::EcmascriptChunkingContextVc;

/// impl of code generation inferred from a AssetReference.
/// This is rust only and can't be implemented by non-rust plugins.
#[turbo_tasks::value(
    shared,
    serialization = "none",
    eq = "manual",
    into = "new",
    cell = "new"
)]
pub struct CodeGeneration {
    /// ast nodes matching the span will be visitor by the visitor
    #[turbo_tasks(debug_ignore, trace_ignore)]
    pub visitors: Vec<(Vec<AstParentKind>, Box<dyn VisitorFactory>)>,
}

pub trait VisitorFactory: Send + Sync {
    fn create<'a>(&'a self) -> Box<dyn VisitMut + Send + Sync + 'a>;
}

#[turbo_tasks::value_trait]
pub trait CodeGenerateable {
    fn code_generation(&self, context: EcmascriptChunkingContextVc) -> CodeGenerationVc;
}

#[turbo_tasks::value_trait]
pub trait CodeGenerateableWithAvailabilityInfo {
    fn code_generation(
        &self,
        context: EcmascriptChunkingContextVc,
        availability_info: Value<AvailabilityInfo>,
    ) -> CodeGenerationVc;
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TraceRawVcs, ValueDebugFormat)]
pub enum CodeGen {
    CodeGenerateable(CodeGenerateableVc),
    CodeGenerateableWithAvailabilityInfo(CodeGenerateableWithAvailabilityInfoVc),
}

#[turbo_tasks::value(transparent)]
pub struct CodeGenerateables(Vec<CodeGen>);

pub fn path_to(
    path: &[AstParentKind],
    f: impl FnMut(&AstParentKind) -> bool,
) -> Vec<AstParentKind> {
    if let Some(pos) = path.iter().rev().position(f) {
        let index = path.len() - pos - 1;
        path[..index].to_vec()
    } else {
        path.to_vec()
    }
}

/// Creates a single-method visitor that will visit the AST nodes matching the
/// provided path.
///
/// If you pass in `exact`, the visitor will only visit the nodes that match the
/// path exactly. Otherwise, the visitor will visit the closest matching parent
/// node in the path.
///
/// Refer to the [swc_core::ecma::visit::VisitMut] trait for a list of all
/// possible visit methods.
#[macro_export]
macro_rules! create_visitor {
    // This rule needs to be first, otherwise we run into the following error:
    // expected one of `!`, `)`, `,`, `.`, `::`, `?`, `{`, or an operator, found `:`
    // This is a regression on nightly.
    (visit_mut_program($arg:ident: &mut Program) $b:block) => {{
        struct Visitor<T: Fn(&mut swc_core::ecma::ast::Program) + Send + Sync> {
            visit_mut_program: T,
        }

        impl<T: Fn(&mut swc_core::ecma::ast::Program) + Send + Sync> $crate::code_gen::VisitorFactory
            for Box<Visitor<T>>
        {
            fn create<'a>(&'a self) -> Box<dyn swc_core::ecma::visit::VisitMut + Send + Sync + 'a> {
                Box::new(&**self)
            }
        }

        impl<'a, T: Fn(&mut swc_core::ecma::ast::Program) + Send + Sync> swc_core::ecma::visit::VisitMut
            for &'a Visitor<T>
        {
            fn visit_mut_program(&mut self, $arg: &mut swc_core::ecma::ast::Program) {
                (self.visit_mut_program)($arg);
            }
        }

        (
            Vec::new(),
            Box::new(Box::new(Visitor {
                visit_mut_program: move |$arg: &mut swc_core::ecma::ast::Program| $b,
            })) as Box<dyn $crate::code_gen::VisitorFactory>,
        )
    }};
    (exact $ast_path:expr, $name:ident($arg:ident: &mut $ty:ident) $b:block) => {
        $crate::create_visitor!(__ $ast_path.to_vec(), $name($arg: &mut $ty) $b)
    };
    ($ast_path:expr, $name:ident($arg:ident: &mut $ty:ident) $b:block) => {
        $crate::create_visitor!(__ $crate::code_gen::path_to(&$ast_path, |n| {
            matches!(n, swc_core::ecma::visit::AstParentKind::$ty(_))
        }), $name($arg: &mut $ty) $b)
    };
    (__ $ast_path:expr, $name:ident($arg:ident: &mut $ty:ident) $b:block) => {{
        struct Visitor<T: Fn(&mut swc_core::ecma::ast::$ty) + Send + Sync> {
            $name: T,
        }

        impl<T: Fn(&mut swc_core::ecma::ast::$ty) + Send + Sync> $crate::code_gen::VisitorFactory
            for Box<Visitor<T>>
        {
            fn create<'a>(&'a self) -> Box<dyn swc_core::ecma::visit::VisitMut + Send + Sync + 'a> {
                Box::new(&**self)
            }
        }

        impl<'a, T: Fn(&mut swc_core::ecma::ast::$ty) + Send + Sync> swc_core::ecma::visit::VisitMut
            for &'a Visitor<T>
        {
            fn $name(&mut self, $arg: &mut swc_core::ecma::ast::$ty) {
                (self.$name)($arg);
            }
        }

        (
            $ast_path,
            Box::new(Box::new(Visitor {
                $name: move |$arg: &mut swc_core::ecma::ast::$ty| $b,
            })) as Box<dyn $crate::code_gen::VisitorFactory>,
        )
    }};
}
