// Copyright 2014 The Servo Project Developers. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use syntax::codemap::Span;
use syntax::ast::{TokenTree, TTTok};
use syntax::ast;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr};
use syntax::parse::token::{get_ident, InternedString, LIT_STR, IDENT};

use std::iter::Chain;
use std::slice::{Items, Found, NotFound};
use std::gc::Gc;
use std::collections::HashMap;
use std::ascii::AsciiExt;

mod data;

#[path="../../../shared/static_atom.rs"]
mod static_atom;

// Build a PhfOrderedSet of static atoms.
// Takes no arguments.
pub fn expand_static_atom_set(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    bail_if!(tt.len() != 0, cx, sp, "Usage: static_atom_map!()");
    let tts: Vec<TokenTree> = data::atoms.iter().flat_map(|k| {
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

// Build a map from atoms to IDs for use in implementing the atom!() macro.
lazy_static! {
    static ref STATIC_ATOM_MAP: HashMap<&'static str, uint> = {
        let mut m = HashMap::new();
        for (i, x) in data::atoms.iter().enumerate() {
            m.insert(*x, i);
        }
        m
    };
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

fn expand_atom_str(cx: &mut ExtCtxt, sp: Span, name: &str) -> Box<MacResult+'static> {
    let i = expect!(cx, sp, STATIC_ATOM_MAP.find_equiv(&name),
        format!("Unknown static atom {:s}", name).as_slice());

    let data = static_atom::add_tag(*i as u32);

    box AtomResult {
        expr: quote_expr!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
        pat: quote_pat!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
    } as Box<MacResult>
}

// Translate `atom!(title)` or `atom!("font-weight")` into an `Atom` constant or pattern.
pub fn expand_atom(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    let usage = "Usage: atom!(html) or atom!(\"font-weight\")";
    let name = match tt {
        [ref t] => expect!(cx, sp, atom_tok_to_str(t), usage),
        _ => bail!(cx, sp, usage),
    };
    expand_atom_str(cx, sp, name.get())
}

// Translate `ns!(HTML)` into `atom!("http://www.w3.org/1999/xhtml")`.
// The argument is ASCII-case-insensitive.
pub fn expand_ns(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    static all_ns: &'static [(&'static str, &'static str)] = [
        ("html", "http://www.w3.org/1999/xhtml"),
        ("xml", "http://www.w3.org/XML/1998/namespace"),
        ("xmlns", "http://www.w3.org/2000/xmlns/"),
        ("xlink", "http://www.w3.org/1999/xlink"),
        ("svg", "http://www.w3.org/2000/svg"),
        ("mathml", "http://www.w3.org/1998/Math/MathML"),
    ];

    let name = match tt {
        [ref t] => atom_tok_to_str(t),
        _ => None,
    };

    match name {
        Some(name) => {
            for &(short, url) in all_ns.iter() {
                if short.eq_ignore_ascii_case(name.get()) {
                    return expand_atom_str(cx, sp, url);
                }
            }
        }
        None => (),
    }

    let ns_names: Vec<&'static str> = all_ns.iter().map(|&(x, _)| x).collect();
    bail!(cx, sp, format!("Usage: ns!(HTML), case-insensitive. Known namespaces: {:s}",
        ns_names.connect(" ")).as_slice());
}
