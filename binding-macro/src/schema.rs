use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, Ident, PathArguments, Type};

fn extract_ident_from_vec(ty: &Type) -> Ident {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .first()
            .expect("should contain type")
            .ident
            .clone(),
        _ => panic!("ty should be Path"),
    }
}

fn extract_ident_from_ty(ty: &Type) -> (Ident, bool) {
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
                            (ident, true)
                        } else {
                            panic!("arg should by Type")
                        }
                    } else {
                        panic!("only support AngleBracketed")
                    }
                } else {
                    if let PathArguments::None = concrete_ty.arguments {
                        (concrete_ty.ident.clone(), false)
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

    let mut tokens = quote! {
        if register.contains_key(#ident_str) {
            return;
        }

        let mut schema = format!("type {} {}\n", #ident_str, "{");
    };

    match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                for f in fields.named.iter() {
                    let field_ident = f.ident.as_ref().expect("field should be named");
                    let field_str = format!("{}", field_ident);

                    let ret = extract_ident_from_ty(&f.ty);
                    let ty_ident = ret.0;
                    let ty_str = ty_ident.to_string();
                    if ret.1 {
                        let token = quote! {
                            if #ty_ident::is_scalar() {
                                let scalar_name = #ty_ident::scalar_name();
                                let it_str = format!("  {}: [{}!]!\n", #field_str, scalar_name);
                                schema.push_str(it_str.as_str());
                            } else {
                                let it_str = format!("  {}: [{}!]!\n", #field_str, #ty_str);
                                schema.push_str(it_str.as_str());
                            }
                            #ty_ident::schema(register);
                        };
                        tokens = quote! {
                            #tokens
                            #token
                        };
                    } else {
                        let token = quote! {
                            if #ty_ident::is_scalar() {
                                let scalar_name = #ty_ident::scalar_name();
                                let it_str = format!("  {}: {}!\n", #field_str, scalar_name);
                                schema.push_str(it_str.as_str());
                            } else {
                                let it_str = format!("  {}: {}!\n", #field_str, #ty_str);
                                schema.push_str(it_str.as_str());
                            }
                            #ty_ident::schema(register);
                        };
                        tokens = quote! {
                            #tokens
                            #token
                        };
                    }
                }
            }
            _ => panic!("struct field should be named"),
        },
        _ => panic!("only struct"),
    }

    let token = quote! {
        schema.push_str("}");
        register.insert(#ident_str.to_owned(), schema);
    };

    tokens = quote! {
        #tokens
        #token
    };

    let gen = quote! {
        impl ServiceSchema for #ident {
            fn is_scalar() -> bool {
                false
            }

            fn scalar_name() -> String {
                "".to_owned()
            }

            fn schema(register: &mut HashMap<String, String>) {
                #tokens
            }
        }
    };

    gen.into()
}
