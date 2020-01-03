use crate::ty::Type;
use std::{borrow::Cow, iter::once};
use swc_atoms::JsWord;
use swc_common::{Spanned, Visit};
use swc_ecma_ast::*;

pub trait ResultExt<T, E>: Into<Result<T, E>> {
    fn store<V>(self, to: &mut V) -> Option<T>
    where
        V: Extend<E>,
    {
        match self.into() {
            Ok(val) => Some(val),
            Err(e) => {
                to.extend(once(e));
                None
            }
        }
    }
}

impl<T, E> ResultExt<T, E> for Result<T, E> {}

/// Simple utility to check (l, r) and (r, l) with same code.
#[derive(Debug, Clone, Copy)]
pub(super) struct Comparator<T>
where
    T: Copy,
{
    pub left: T,
    pub right: T,
}

impl<T> Comparator<T>
where
    T: Copy,
{
    pub fn take<F, R>(&self, mut op: F) -> Option<R>
    where
        F: FnMut(T, T) -> Option<R>,
    {
        op(self.left, self.right).or_else(|| op(self.right, self.left))
    }

    pub fn both<F>(&self, mut op: F) -> bool
    where
        F: FnMut(T) -> bool,
    {
        op(self.left) && op(self.right)
    }

    pub fn any<F>(&self, mut op: F) -> bool
    where
        F: FnMut(T) -> bool,
    {
        op(self.left) || op(self.right)
    }
}

pub(super) fn is_prop_name_eq(l: &PropName, r: &PropName) -> bool {
    macro_rules! check {
        ($l:expr, $r:expr) => {{
            let l = $l;
            let r = $r;

            match l {
                PropName::Ident(Ident { ref sym, .. })
                | PropName::Str(Str { value: ref sym, .. }) => match *r {
                    PropName::Ident(Ident { sym: ref r_sym, .. })
                    | PropName::Str(Str {
                        value: ref r_sym, ..
                    }) => return sym == r_sym,
                    PropName::Num(n) => return sym == &*n.value.to_string(),
                    _ => return false,
                },
                PropName::Computed(..) => return false,
                _ => {}
            }
        }};
    }

    check!(l, r);
    check!(r, l);

    false
}

pub(super) trait PatExt {
    fn get_ty(&self) -> Option<&TsType>;
    fn set_ty(&mut self, ty: Option<Box<TsType>>);
}

impl PatExt for Pat {
    fn get_ty(&self) -> Option<&TsType> {
        match *self {
            Pat::Array(ArrayPat { ref type_ann, .. })
            | Pat::Assign(AssignPat { ref type_ann, .. })
            | Pat::Ident(Ident { ref type_ann, .. })
            | Pat::Object(ObjectPat { ref type_ann, .. })
            | Pat::Rest(RestPat { ref type_ann, .. }) => type_ann.as_ref().map(|ty| &*ty.type_ann),

            _ => unreachable!("Cannot get type from {:?}", self),
        }
    }

    fn set_ty(&mut self, ty: Option<Box<TsType>>) {
        match *self {
            Pat::Array(ArrayPat {
                ref mut type_ann, ..
            })
            | Pat::Assign(AssignPat {
                ref mut type_ann, ..
            })
            | Pat::Ident(Ident {
                ref mut type_ann, ..
            })
            | Pat::Object(ObjectPat {
                ref mut type_ann, ..
            })
            | Pat::Rest(RestPat {
                ref mut type_ann, ..
            }) => {
                *type_ann = ty.map(|type_ann| TsTypeAnn {
                    span: type_ann.span(),
                    type_ann,
                })
            }

            _ => unreachable!("Cannot set type annotations for {:?}", self),
        }
    }
}

pub trait NormalizeMut<'b> {
    fn normalize_mut(&mut self) -> &mut Type<'b>;
}

impl<'b, T> NormalizeMut<'b> for Box<T>
where
    T: NormalizeMut<'b>,
{
    fn normalize_mut(&mut self) -> &mut Type<'b> {
        self.as_mut().normalize_mut()
    }
}

impl<'b, T> NormalizeMut<'b> for &'_ mut T
where
    T: NormalizeMut<'b>,
{
    fn normalize_mut(&mut self) -> &mut Type<'b> {
        (*self).normalize_mut()
    }
}

impl<'a, 'b> NormalizeMut<'b> for Cow<'a, Type<'b>> {
    fn normalize_mut(&mut self) -> &mut Type<'b> {
        let owned = match *self {
            Cow::Borrowed(borrowed) => {
                *self = Cow::Owned(borrowed.to_owned());
                match *self {
                    Cow::Borrowed(..) => unreachable!(),
                    Cow::Owned(ref mut owned) => owned,
                }
            }
            Cow::Owned(ref mut owned) => owned,
        };

        match *owned {
            Type::Static(s) => {
                *owned = s.ty.clone().owned().into_owned();
                owned
            }

            _ => owned,
        }
    }
}

pub(super) struct VarVisitor<'a> {
    pub names: &'a mut Vec<JsWord>,
}

impl Visit<Expr> for VarVisitor<'_> {
    fn visit(&mut self, _: &Expr) {}
}

impl Visit<Ident> for VarVisitor<'_> {
    fn visit(&mut self, i: &Ident) {
        self.names.push(i.sym.clone())
    }
}
