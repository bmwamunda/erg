use std::fmt;

use erg_common::dict::Dict;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::Str;

use erg_parser::ast::{ConstBlock, Params};

use super::constructors::subr_t;
use super::value::{EvalValueResult, ValueObj};
use super::{Predicate, Type};

use crate::context::Context;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserConstSubr {
    name: Str,
    params: Params,
    block: ConstBlock,
    sig_t: Type,
}

impl UserConstSubr {
    pub const fn new(name: Str, params: Params, block: ConstBlock, sig_t: Type) -> Self {
        Self {
            name,
            params,
            block,
            sig_t,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueArgs {
    pub pos_args: Vec<ValueObj>,
    pub kw_args: Dict<Str, ValueObj>,
}

impl ValueArgs {
    pub const fn new(pos_args: Vec<ValueObj>, kw_args: Dict<Str, ValueObj>) -> Self {
        ValueArgs { pos_args, kw_args }
    }

    pub fn remove_left_or_key(&mut self, key: &str) -> Option<ValueObj> {
        if !self.pos_args.is_empty() {
            Some(self.pos_args.remove(0))
        } else {
            self.kw_args.remove(key)
        }
    }
}

#[derive(Clone)]
pub struct BuiltinConstSubr {
    name: &'static str,
    subr: fn(ValueArgs, &Context) -> EvalValueResult<ValueObj>,
    sig_t: Type,
    as_type: Option<Type>,
}

impl std::fmt::Debug for BuiltinConstSubr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinConstSubr")
            .field("name", &self.name)
            .field("sig_t", &self.sig_t)
            .field("as_type", &self.as_type)
            .finish()
    }
}

impl PartialEq for BuiltinConstSubr {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for BuiltinConstSubr {}

impl std::hash::Hash for BuiltinConstSubr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl fmt::Display for BuiltinConstSubr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<built-in const subroutine '{}'>", self.name)
    }
}

impl BuiltinConstSubr {
    pub const fn new(
        name: &'static str,
        subr: fn(ValueArgs, &Context) -> EvalValueResult<ValueObj>,
        sig_t: Type,
        as_type: Option<Type>,
    ) -> Self {
        Self {
            name,
            subr,
            sig_t,
            as_type,
        }
    }

    pub fn call(&self, args: ValueArgs, ctx: &Context) -> EvalValueResult<ValueObj> {
        (self.subr)(args, ctx)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstSubr {
    User(UserConstSubr),
    Builtin(BuiltinConstSubr),
}

impl fmt::Display for ConstSubr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstSubr::User(subr) => {
                write!(f, "<user-defined const subroutine '{}'>", subr.name)
            }
            ConstSubr::Builtin(subr) => write!(f, "{subr}"),
        }
    }
}

impl ConstSubr {
    pub fn sig_t(&self) -> &Type {
        match self {
            ConstSubr::User(user) => &user.sig_t,
            ConstSubr::Builtin(builtin) => &builtin.sig_t,
        }
    }

    /// ConstSubr{sig_t: Int -> {Int}, ..}.as_type() == Int -> Int
    pub fn as_type(&self, ctx: &Context) -> Option<Type> {
        match self {
            ConstSubr::User(user) => {
                // TODO: quantified types
                let subr = match &user.sig_t {
                    Type::Subr(subr) => subr,
                    Type::Quantified(subr) => {
                        if let Type::Subr(subr) = subr.as_ref() {
                            subr
                        } else {
                            return None;
                        }
                    }
                    _ => {
                        return None;
                    }
                };
                if let Type::Refinement(refine) = subr.return_t.as_ref() {
                    if let Predicate::Equal { rhs, .. } = refine.pred.as_ref() {
                        let return_t = ctx.convert_tp_into_type(rhs.clone()).ok()?;
                        let var_params = subr.var_params.as_ref().map(|t| t.as_ref());
                        let subr_t = subr_t(
                            subr.kind,
                            subr.non_default_params.clone(),
                            var_params.cloned(),
                            subr.default_params.clone(),
                            return_t,
                        );
                        return Some(subr_t);
                    }
                }
                None
            }
            ConstSubr::Builtin(builtin) => builtin.as_type.clone(),
        }
    }
}
