// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! **Note:** This may move as string-cache becomes less Web-specific.

use atom::Atom;

/// An atom that is meant to represent a namespace in the HTML / XML sense.
/// Whether a given string represents a namespace is contextual, so this is
/// a transparent wrapper that will not catch all mistakes.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct Namespace(pub Atom);

/// A name with a namespace.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct QualName {
    pub ns: Namespace,
    pub local: Atom,
}

impl QualName {
    #[inline]
    pub fn new(ns: Namespace, local: Atom) -> QualName {
        QualName {
            ns: ns,
            local: local,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Namespace, QualName};

    #[test]
    fn ns_macro() {
        assert_eq!(ns!(""),     Namespace(atom!("")));

        assert_eq!(ns!(html),   Namespace(atom!("http://www.w3.org/1999/xhtml")));
        assert_eq!(ns!(xml),    Namespace(atom!("http://www.w3.org/XML/1998/namespace")));
        assert_eq!(ns!(xmlns),  Namespace(atom!("http://www.w3.org/2000/xmlns/")));
        assert_eq!(ns!(xlink),  Namespace(atom!("http://www.w3.org/1999/xlink")));
        assert_eq!(ns!(svg),    Namespace(atom!("http://www.w3.org/2000/svg")));
        assert_eq!(ns!(mathml), Namespace(atom!("http://www.w3.org/1998/Math/MathML")));

        assert_eq!(ns!(HtMl),   Namespace(atom!("http://www.w3.org/1999/xhtml")));
        assert_eq!(ns!(xMl),    Namespace(atom!("http://www.w3.org/XML/1998/namespace")));
        assert_eq!(ns!(XmLnS),  Namespace(atom!("http://www.w3.org/2000/xmlns/")));
        assert_eq!(ns!(xLiNk),  Namespace(atom!("http://www.w3.org/1999/xlink")));
        assert_eq!(ns!(SvG),    Namespace(atom!("http://www.w3.org/2000/svg")));
        assert_eq!(ns!(mAtHmL), Namespace(atom!("http://www.w3.org/1998/Math/MathML")));
    }

    #[test]
    fn qualname() {
        assert_eq!(QualName::new(ns!(""), atom!("")),
            QualName { ns: ns!(""), local: atom!("") });
        assert_eq!(QualName::new(ns!(XML), atom!(base)),
            QualName { ns: ns!(XML), local: atom!(base) });
    }

    #[test]
    fn qualname_macro() {
        assert_eq!(qualname!("", ""), QualName { ns: ns!(""), local: atom!("") });
        assert_eq!(qualname!(XML, base), QualName { ns: ns!(XML), local: atom!(base) });
    }
}
