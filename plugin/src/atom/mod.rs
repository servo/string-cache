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
use syntax::ast::{TokenTree, TtToken};
use syntax::ast;
use syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use syntax::parse::token::{get_ident, InternedString, Ident, Literal, Lit};

use std::iter::Chain;
use std::collections::HashMap;
use std::ascii::AsciiExt;


fn atom_tok_to_str(t: &TokenTree) -> Option<InternedString> {
    Some(get_ident(match *t {
        TtToken(_, Ident(s, _)) => s,
        TtToken(_, Literal(Lit::Str_(s), _)) => s.ident(),
        _ => return None,
    }))
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
    let i = match ::string_cache_shared::STATIC_ATOM_SET.get_index(name) {
        Some(i) => i,
        None => return None,
    };

    let data = ::string_cache_shared::pack_static(i as u32);

    Some(AtomResult {
        expr: quote_expr!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
        pat: quote_pat!(&mut *cx, ::string_cache::atom::Atom { data: $data }),
    })
}

// Translate `atom!(title)` or `atom!("font-weight")` into an `Atom` constant or pattern.
pub fn expand_atom(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    let usage = "Usage: atom!(html) or atom!(\"font-weight\")";
    let name = match tt {
        [ref t] => ext_expect!(cx, sp, atom_tok_to_str(t), usage),
        _ => ext_bail!(cx, sp, usage),
    };
    box ext_expect!(cx, sp, make_atom_result(cx, &*name),
        &format!("Unknown static atom {}", &*name))
}

// Translate `ns!(HTML)` into `Namespace { atom: atom!("http://www.w3.org/1999/xhtml") }`.
// The argument is ASCII-case-insensitive.
pub fn expand_ns(cx: &mut ExtCtxt, sp: Span, tt: &[TokenTree]) -> Box<MacResult+'static> {
    use string_cache_shared::ALL_NS;

    fn usage() -> String {
        let ns_names: Vec<&'static str> = ALL_NS[1..].iter()
            .map(|&(x, _)| x).collect();
        format!("Usage: ns!(HTML), case-insensitive. \
            Known namespaces: {}",
            ns_names.join(" "))
    }

    let name = ext_expect!(cx, sp, match tt {
        [ref t] => atom_tok_to_str(t),
        _ => None,
    }, &usage());

    let &(_, url) = ext_expect!(cx, sp,
        ALL_NS.iter().find(|&&(short, _)| short.eq_ignore_ascii_case(&*name)),
        &usage());

    // All of the URLs should be in the static atom table.
    let AtomResult { expr, pat } = ext_expect!(cx, sp, make_atom_result(cx, url),
        &format!("internal plugin error: can't find namespace url {}", url));

    box AtomResult {
        expr: quote_expr!(&mut *cx, ::string_cache::namespace::Namespace($expr)),
        pat: quote_pat!(&mut *cx, ::string_cache::namespace::Namespace($pat)),
    }
}
