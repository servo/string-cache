// Copyright 2014 The Servo Project Developers. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name="string_cache_macros"]
#![crate_type="dylib"]

#![feature(macro_rules, plugin_registrar, quote, managed_boxes)]
#![allow(unused_imports)]  // for quotes

extern crate syntax;
extern crate rustc;

use rustc::plugin::Registry;
use syntax::codemap::Span;
use syntax::ast::{TokenTree, TTTok};
use syntax::ast;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr};
use syntax::parse::token::{get_ident, InternedString, LIT_STR, IDENT};

use std::iter::Chain;
use std::slice::Items;
use std::gc::Gc;

mod data;

#[path="../../shared/static_atom.rs"]
mod static_atom;

macro_rules! bail ( ($cx:expr, $sp:expr, $msg:expr) => ({
    $cx.span_err($sp, $msg);
    return ::syntax::ext::base::DummyResult::any($sp);
}))

macro_rules! bail_if ( ($e:expr, $cx:expr, $sp:expr, $msg:expr) => (
    if $e { bail!($cx, $sp, $msg) }
))

macro_rules! expect ( ($cx:expr, $sp:expr, $e:expr, $msg:expr) => (
    match $e {
        Some(x) => x,
        None => bail!($cx, $sp, $msg),
    }
))

// Build a PhfOrderedSet of static atoms.
// Takes no arguments.
fn expand_static_atom_set(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    bail_if!(tt.len() != 0, cx, sp, "Usage: static_atom_map!()");
    let all_atoms = data::fast_set_atoms.iter().chain(data::other_atoms.iter());
    let tts: Vec<TokenTree> = all_atoms.flat_map(|k| {
        (quote_tokens!(&mut *cx, $k,)).move_iter()
    }).collect();
    MacExpr::new(quote_expr!(&mut *cx, phf_ordered_set!($tts)))
}

fn atom_tok_to_str(t: &TokenTree) -> Option<InternedString> {
    Some(get_ident(match *t {
        TTTok(_, IDENT(s, _)) => s,
        TTTok(_, LIT_STR(s)) => s.ident(),
        _ => return None,
    }))
}

fn find_atom(name: InternedString) -> Option<uint> {
    // Use bsearch instead of bsearch_elem because of type mismatch
    // between &'t str and &'static str.
    data::fast_set_atoms.bsearch(|&x| x.cmp(&name.get())).or_else(||
        data::other_atoms.bsearch(|&x| x.cmp(&name.get())).map(|i| i+64))

}

struct AtomResult {
    expr: Gc<ast::Expr>,
    pat: Gc<ast::Pat>,
}

impl MacResult for AtomResult {
    fn make_expr(&self) -> Option<Gc<ast::Expr>> {
        Some(self.expr)
    }

    fn make_pat(&self) -> Option<Gc<ast::Pat>> {
        Some(self.pat)
    }
}

// Translate `atom!(title)` or `atom!("font-weight")` into an `Atom` constant or pattern.
fn expand_atom(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    let usage = "Usage: atom!(html) or atom!(\"font-weight\")";
    let name = match tt {
        [ref t] => expect!(cx, sp, atom_tok_to_str(t), usage),
        _ => bail!(cx, sp, usage),
    };

    let i = expect!(cx, sp, find_atom(name.clone()),
        format!("Unknown static atom {:s}", name.get()).as_slice());

    let data = static_atom::add_tag(i as u32);

    box AtomResult {
        expr: quote_expr!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
        pat: quote_pat!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
    } as Box<MacResult>
}

// NB: This needs to be public or we get a linker error.
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("static_atom_set", expand_static_atom_set);
    reg.register_macro("atom", expand_atom);
}
