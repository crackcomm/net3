use proc_macro2::TokenStream as TokenStream2;
use syn::{punctuated::Punctuated, *};

use darling::FromMeta;

#[derive(Default, Debug, FromMeta)]
pub struct MethodAttr {
    #[darling(default)]
    pub name: Option<String>,
}

pub struct MethodDef {
    pub attr: MethodAttr,
    pub item: TraitItemMethod,
    pub definition: TokenStream2,
    pub attributes: Vec<Attribute>,
}

impl MethodDef {
    pub fn name(&self) -> &Ident {
        &self.item.sig.ident
    }

    pub fn signature(&self) -> &Signature {
        &self.item.sig
    }

    pub fn method_name(&self) -> String {
        match self.attr.name.clone() {
            Some(name) => name,
            None => self.name().to_string(),
        }
    }

    pub fn inputs(&self) -> Punctuated<&FnArg, Token![,]> {
        self.signature()
            .inputs
            .iter()
            .skip(1)
            .collect::<Punctuated<_, Token![,]>>()
    }

    pub fn input_names(&self) -> Punctuated<Ident, Token![,]> {
        self.inputs()
            .iter()
            .map(|arg| expect_arg_name(*arg))
            .collect::<Punctuated<_, Token![,]>>()
    }
}

pub fn parse_method_attrs(
    attrs: &[Attribute],
) -> std::result::Result<(MethodAttr, Vec<Attribute>), TokenStream2> {
    let (attr, forward) = attrs
        .iter()
        .fold(Ok((None, Vec::new())), |aggr, attr| match aggr {
            Ok((rpc_attr, mut forward)) => {
                match attr.path.get_ident().expect("ident").to_string().as_str() {
                    "rpc" => {
                        let meta = match attr.parse_meta() {
                            Ok(attr) => attr,
                            Err(err) => return Err(err.to_compile_error()),
                        };
                        match MethodAttr::from_meta(&meta) {
                            Ok(attr) => Ok((Some(attr), forward)),
                            Err(err) => Err(err.write_errors()),
                        }
                    }
                    _ => {
                        forward.push(attr.clone());
                        Ok((rpc_attr, forward))
                    }
                }
            }
            err => err,
        })?;
    let attr = attr.expect("rpc trait method attributes");
    Ok((attr, forward))
}

fn expect_arg_name(arg: &FnArg) -> Ident {
    match arg {
        FnArg::Typed(ty) => match ty.pat.as_ref() {
            Pat::Ident(pat) => pat.ident.clone(),
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}
