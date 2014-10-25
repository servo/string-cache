// Copyright 2014 The Servo Project Developers. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use syntax::ptr::P;
use syntax::codemap::Span;
use syntax::ast::{TokenTree, TTTok};
use syntax::ast;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr};
use syntax::parse::token::{get_ident, InternedString, LIT_STR, IDENT};

use std::iter::Chain;
use std::slice::{Items, Found, NotFound};
use std::collections::HashMap;
use std::ascii::AsciiExt;

mod data;

#[path="../../../shared/repr.rs"]
mod repr;

// Build a PhfOrderedSet of static atoms.
// Takes no arguments.
pub fn expand_static_atom_set(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    bail_if!(tt.len() != 0, cx, sp, "Usage: static_atom_map!()");
    let tts: Vec<TokenTree> = data::ATOMS.iter().flat_map(|k| {
        (quote_tokens!(&mut *cx, $k,)).into_iter()
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
        for (i, x) in data::ATOMS.iter().enumerate() {
            m.insert(*x, i);
        }
        m
    };
}

// FIXME: libsyntax should provide this (rust-lang/rust#17637)
struct AtomResult {
    expr: P<ast::Expr>,
    pat: P<ast::Pat>,
}

impl MacResult for AtomResult {
    fn make_expr(self: Box<AtomResult>) -> Option<P<ast::Expr>> {
        Some(self.expr)
    }

    fn make_pat(self: Box<AtomResult>) -> Option<P<ast::Pat>> {
        Some(self.pat)
    }
}

fn make_atom_result(cx: &mut ExtCtxt, name: &str) -> Option<AtomResult> {
    let i = match STATIC_ATOM_MAP.find_equiv(&name) {
        Some(i) => i,
        None => return None,
    };

    let data = repr::pack_static(*i as u32);

    Some(AtomResult {
        expr: quote_expr!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
        pat: quote_pat!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
    })
}

// Translate `atom!(title)` or `atom!("font-weight")` into an `Atom` constant or pattern.
pub fn expand_atom(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    let usage = "Usage: atom!(html) or atom!(\"font-weight\")";
    let name = match tt {
        [ref t] => expect!(cx, sp, atom_tok_to_str(t), usage),
        _ => bail!(cx, sp, usage),
    };
    box expect!(cx, sp, make_atom_result(cx, name.get()),
        format!("Unknown static atom {:s}", name.get()).as_slice())
}

// Translate `ns!(HTML)` into `Namespace { atom: atom!("http://www.w3.org/1999/xhtml") }`.
// The argument is ASCII-case-insensitive.
pub fn expand_ns(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    static ALL_NS: &'static [(&'static str, &'static str)] = [
        ("", ""),
        ("html", "http://www.w3.org/1999/xhtml"),
        ("xml", "http://www.w3.org/XML/1998/namespace"),
        ("xmlns", "http://www.w3.org/2000/xmlns/"),
        ("xlink", "http://www.w3.org/1999/xlink"),
        ("svg", "http://www.w3.org/2000/svg"),
        ("mathml", "http://www.w3.org/1998/Math/MathML"),
    ];

    fn usage() -> String {
        let ns_names: Vec<&'static str> = ALL_NS.slice_from(1).iter()
            .map(|&(x, _)| x).collect();
        format!("Usage: ns!(HTML), case-insensitive. \
            Known namespaces: {:s}",
            ns_names.connect(" "))
    }

    let name = expect!(cx, sp, match tt {
        [ref t] => atom_tok_to_str(t),
        _ => None,
    }, usage().as_slice());

    let &(_, url) = expect!(cx, sp,
        ALL_NS.iter().find(|&&(short, _)| short.eq_ignore_ascii_case(name.get())),
        usage().as_slice());

    // All of the URLs should be in the static atom table.
    let AtomResult { expr, pat } = expect!(cx, sp, make_atom_result(cx, url),
        format!("internal plugin error: can't find namespace url {:s}", url).as_slice());

    box AtomResult {
        expr: quote_expr!(&mut *cx, ::string_cache::namespace::Namespace($expr)),
        pat: quote_pat!(&mut *cx, ::string_cache::namespace::Namespace($pat)),
    }
}

#[macro_export]
macro_rules! qualname (($ns:tt, $local:tt) => (
    ::string_cache::namespace::QualName {
        ns: ns!($ns),
        local: atom!($local),
    }
))
