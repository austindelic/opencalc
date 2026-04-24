use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::expr::Expr;

#[derive(Clone, Debug)]
pub struct UserFn {
    pub params: Vec<String>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct Env {
    pub vars: BTreeMap<String, Expr>,
    pub fns:  BTreeMap<String, UserFn>,
}

impl Env {
    pub fn new() -> Self {
        Env { vars: BTreeMap::new(), fns: BTreeMap::new() }
    }

    pub fn set_var(&mut self, name: &str, val: Expr) {
        self.vars.insert(name.to_string(), val);
    }

    pub fn get_var(&self, name: &str) -> Option<&Expr> {
        self.vars.get(name)
    }

    pub fn set_fn(&mut self, name: &str, f: UserFn) {
        self.fns.insert(name.to_string(), f);
    }

    pub fn get_fn(&self, name: &str) -> Option<&UserFn> {
        self.fns.get(name)
    }
}

impl Default for Env {
    fn default() -> Self { Env::new() }
}
