use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};

fn mark_scalar(ident: &str) -> (String, u8) {
    match ident {
        "Address" => ("Address".to_owned(), 1),
        "Hash" => ("Hash".to_owned(), 2),
        "Bytes" => ("Bytes".to_owned(), 4),
        "u32" => ("Uint32".to_owned(), 8),
        "u64" => ("Uint64".to_owned(), 16),
        "Hex" => ("Hex".to_owned(), 32),
        _ => (ident.to_owned(), 0),
    }
}

fn extract_ident_from_vec(ty: &Type) -> String {
    match ty {
        Type::Path(path) => {
            let ident = &path
                .path
                .segments
                .first()
                .expect("should contain type")
                .ident;
            format!("{}", ident)
        }
        _ => panic!("ty should be Path"),
    }
}

fn extract_ident_from_ty(ty: &Type) -> (String, u8) {
    match ty {
        Type::Path(ty) => {
            let segs = &ty.path.segments;
            if 1 == segs.len() {
                let concrete_ty = segs.first().unwrap();
                if "Vec".to_owned() == format!("{}", &concrete_ty.ident) {
                    if let PathArguments::AngleBracketed(g_ty) = &concrete_ty.arguments {
                        let arg = g_ty.args.first().expect("should contain arg");
                        if let GenericArgument::Type(arg_ty) = arg {
                            let ident = extract_ident_from_vec(&arg_ty);
                            let ret = mark_scalar(ident.as_str());
                            (format!("[{}!]!\n", ret.0), ret.1)
                        } else {
                            panic!("arg should by Type")
                        }
                    } else {
                        panic!("only support AngleBracketed")
                    }
                } else {
                    if let PathArguments::None = concrete_ty.arguments {
                        let ident = format!("{}", concrete_ty.ident);
                        let ret = mark_scalar(ident.as_str());
                        (format!("{}!\n", ret.0), ret.1)
                    } else {
                        panic!("only support Vec")
                    }
                }
            } else {
                panic!("only support length 1")
            }
        }
        _ => panic!("only support path"),
    }
}

pub fn impl_service_input(ast: &DeriveInput) -> TokenStream {
    let ident = &ast.ident;
    let ident_str = format!("{}", ident);
    let mut fields_str: String = "".to_owned();
    let mut scalar: u8 = 0;

    match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                for f in fields.named.iter() {
                    let field_ident = f.ident.as_ref().expect("field should be named");
                    let name_str = format!("    {}: ", field_ident);
                    let ret = extract_ident_from_ty(&f.ty);
                    let s = name_str + ret.0.as_str();
                    fields_str.push_str(s.as_str());
                    scalar = scalar | ret.1;
                }
            }
            _ => panic!("struct field should be named"),
        },
        _ => panic!("only struct"),
    }

    let gen = quote! {
        impl ServiceSchema for #ident {
            fn get_schema() -> (String, u8) {
                (format!("type {} {}\n{}{}", #ident_str, "{", #fields_str, "}"), #scalar)
            }
        }
    };

    gen.into()
}
