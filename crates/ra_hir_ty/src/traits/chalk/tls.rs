//! Implementation of Chalk debug helper functions using TLS.
use std::fmt;

use chalk_ir::{AliasTy, Goal, Goals, Lifetime, Parameter, ProgramClauseImplication, TypeName};

use super::{from_chalk, Interner};
use crate::{db::HirDatabase, CallableDef, TypeCtor};
use hir_def::{AdtId, AssocContainerId, DefWithBodyId, Lookup, TypeAliasId};

pub use unsafe_tls::{set_current_program, with_current_program};

pub struct DebugContext<'a>(&'a (dyn HirDatabase + 'a));

impl DebugContext<'_> {
    pub fn debug_struct_id(
        &self,
        id: super::StructId,
        f: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        let type_ctor: TypeCtor = from_chalk(self.0, TypeName::Struct(id));
        match type_ctor {
            TypeCtor::Bool => write!(f, "bool")?,
            TypeCtor::Char => write!(f, "char")?,
            TypeCtor::Int(t) => write!(f, "{}", t)?,
            TypeCtor::Float(t) => write!(f, "{}", t)?,
            TypeCtor::Str => write!(f, "str")?,
            TypeCtor::Slice => write!(f, "slice")?,
            TypeCtor::Array => write!(f, "array")?,
            TypeCtor::RawPtr(m) => write!(f, "*{}", m.as_keyword_for_ptr())?,
            TypeCtor::Ref(m) => write!(f, "&{}", m.as_keyword_for_ref())?,
            TypeCtor::Never => write!(f, "!")?,
            TypeCtor::Tuple { .. } => {
                write!(f, "()")?;
            }
            TypeCtor::FnPtr { .. } => {
                write!(f, "fn")?;
            }
            TypeCtor::FnDef(def) => {
                let name = match def {
                    CallableDef::FunctionId(ff) => self.0.function_data(ff).name.clone(),
                    CallableDef::StructId(s) => self.0.struct_data(s).name.clone(),
                    CallableDef::EnumVariantId(e) => {
                        let enum_data = self.0.enum_data(e.parent);
                        enum_data.variants[e.local_id].name.clone()
                    }
                };
                match def {
                    CallableDef::FunctionId(_) => write!(f, "{{fn {}}}", name)?,
                    CallableDef::StructId(_) | CallableDef::EnumVariantId(_) => {
                        write!(f, "{{ctor {}}}", name)?
                    }
                }
            }
            TypeCtor::Adt(def_id) => {
                let name = match def_id {
                    AdtId::StructId(it) => self.0.struct_data(it).name.clone(),
                    AdtId::UnionId(it) => self.0.union_data(it).name.clone(),
                    AdtId::EnumId(it) => self.0.enum_data(it).name.clone(),
                };
                write!(f, "{}", name)?;
            }
            TypeCtor::AssociatedType(type_alias) => {
                let trait_ = match type_alias.lookup(self.0.upcast()).container {
                    AssocContainerId::TraitId(it) => it,
                    _ => panic!("not an associated type"),
                };
                let trait_name = self.0.trait_data(trait_).name.clone();
                let name = self.0.type_alias_data(type_alias).name.clone();
                write!(f, "{}::{}", trait_name, name)?;
            }
            TypeCtor::Closure { def, expr } => {
                write!(f, "{{closure {:?} in ", expr.into_raw())?;
                match def {
                    DefWithBodyId::FunctionId(func) => {
                        write!(f, "fn {}", self.0.function_data(func).name)?
                    }
                    DefWithBodyId::StaticId(s) => {
                        if let Some(name) = self.0.static_data(s).name.as_ref() {
                            write!(f, "body of static {}", name)?;
                        } else {
                            write!(f, "body of unnamed static {:?}", s)?;
                        }
                    }
                    DefWithBodyId::ConstId(c) => {
                        if let Some(name) = self.0.const_data(c).name.as_ref() {
                            write!(f, "body of const {}", name)?;
                        } else {
                            write!(f, "body of unnamed const {:?}", c)?;
                        }
                    }
                };
                write!(f, "}}")?;
            }
        }
        Ok(())
    }

    pub fn debug_trait_id(
        &self,
        id: super::TraitId,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        let trait_: hir_def::TraitId = from_chalk(self.0, id);
        let trait_data = self.0.trait_data(trait_);
        write!(fmt, "{}", trait_data.name)
    }

    pub fn debug_assoc_type_id(
        &self,
        id: super::AssocTypeId,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        let type_alias: TypeAliasId = from_chalk(self.0, id);
        let type_alias_data = self.0.type_alias_data(type_alias);
        let trait_ = match type_alias.lookup(self.0.upcast()).container {
            AssocContainerId::TraitId(t) => t,
            _ => panic!("associated type not in trait"),
        };
        let trait_data = self.0.trait_data(trait_);
        write!(fmt, "{}::{}", trait_data.name, type_alias_data.name)
    }

    pub fn debug_alias(
        &self,
        alias: &AliasTy<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        let type_alias: TypeAliasId = from_chalk(self.0, alias.associated_ty_id);
        let type_alias_data = self.0.type_alias_data(type_alias);
        let trait_ = match type_alias.lookup(self.0.upcast()).container {
            AssocContainerId::TraitId(t) => t,
            _ => panic!("associated type not in trait"),
        };
        let trait_data = self.0.trait_data(trait_);
        let params = alias.substitution.parameters(&Interner);
        write!(
            fmt,
            "<{:?} as {}<{:?}>>::{}",
            &params[0],
            trait_data.name,
            &params[1..],
            type_alias_data.name
        )
    }

    pub fn debug_ty(
        &self,
        ty: &chalk_ir::Ty<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", ty.data(&Interner))
    }

    pub fn debug_lifetime(
        &self,
        lifetime: &Lifetime<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", lifetime.data(&Interner))
    }

    pub fn debug_parameter(
        &self,
        parameter: &Parameter<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", parameter.data(&Interner).inner_debug())
    }

    pub fn debug_goal(
        &self,
        goal: &Goal<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        let goal_data = goal.data(&Interner);
        write!(fmt, "{:?}", goal_data)
    }

    pub fn debug_goals(
        &self,
        goals: &Goals<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", goals.debug(&Interner))
    }

    pub fn debug_program_clause_implication(
        &self,
        pci: &ProgramClauseImplication<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", pci.debug(&Interner))
    }

    pub fn debug_application_ty(
        &self,
        application_ty: &chalk_ir::ApplicationTy<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", application_ty.debug(&Interner))
    }

    pub fn debug_substitution(
        &self,
        substitution: &chalk_ir::Substitution<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", substitution.debug(&Interner))
    }

    pub fn debug_separator_trait_ref(
        &self,
        separator_trait_ref: &chalk_ir::SeparatorTraitRef<Interner>,
        fmt: &mut fmt::Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}", separator_trait_ref.debug(&Interner))
    }
}

mod unsafe_tls {
    use super::DebugContext;
    use crate::db::HirDatabase;
    use scoped_tls::scoped_thread_local;

    scoped_thread_local!(static PROGRAM: DebugContext);

    pub fn with_current_program<R>(
        op: impl for<'a> FnOnce(Option<&'a DebugContext<'a>>) -> R,
    ) -> R {
        if PROGRAM.is_set() {
            PROGRAM.with(|prog| op(Some(prog)))
        } else {
            op(None)
        }
    }

    pub fn set_current_program<OP, R>(p: &dyn HirDatabase, op: OP) -> R
    where
        OP: FnOnce() -> R,
    {
        let ctx = DebugContext(p);
        // we're transmuting the lifetime in the DebugContext to static. This is
        // fine because we only keep the reference for the lifetime of this
        // function, *and* the only way to access the context is through
        // `with_current_program`, which hides the lifetime through the `for`
        // type.
        let static_p: &DebugContext<'static> =
            unsafe { std::mem::transmute::<&DebugContext, &DebugContext<'static>>(&ctx) };
        PROGRAM.set(static_p, || op())
    }
}
