use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, FnArg, GenericArgument, Ident, ImplItem, ImplItemMethod, ItemImpl,
    PathArguments, ReturnType, Type,
};

const READ_ATTRIBUTE: &str = "read";
const WRITE_ATTRIBUTE: &str = "write";
const GENESIS_ATTRIBUTE: &str = "genesis";
const HOOK_BEFORE_ATTRIBUTE: &str = "hook_before";
const HOOK_AFTER_ATTRIBUTE: &str = "hook_after";
const TX_HOOK_BEFORE_ATTRIBUTE: &str = "tx_hook_before";
const TX_HOOK_AFTER_ATTRIBUTE: &str = "tx_hook_after";

enum ServiceMethod {
    Read(ImplItemMethod),
    Write(ImplItemMethod),
}

struct Hooks {
    before:    Option<Ident>,
    after:     Option<Ident>,
    tx_before: Option<Ident>,
    tx_after:  Option<Ident>,
}

#[derive(Debug)]
struct MethodMeta {
    method_ident:  Ident,
    payload_ident: Option<Ident>,
    readonly:      bool,
    res_ident:     Option<Ident>,
}

fn gen_schema_code(service: &Ident, methods: &Vec<MethodMeta>) -> proc_macro2::TokenStream {
    let mut mutation = format!("type {}Mutation {}\n", service, "{");
    let mut query = format!("type {}Query {}\n", service, "{");

    let mut tokens = quote! {
        let mut register = HashMap::<String, String>::new();
    };

    for m in methods.iter() {
        if m.readonly {
            if m.res_ident.is_none() {
                if m.payload_ident.is_none() {
                    query.push_str(format!("  {}()\n", &m.method_ident).as_str());
                } else {
                    let payload_ident = m.payload_ident.as_ref().unwrap();
                    query.push_str(
                        format!(
                            "  {}(\n    payload: {}!\n  )\n",
                            &m.method_ident, &payload_ident
                        )
                        .as_str(),
                    );
                    let token = quote! {
                        #payload_ident::schema(&mut register);
                    };
                    tokens = quote! {
                        #tokens
                        #token
                    };
                }
            } else {
                if m.payload_ident.is_none() {
                    let res_ident = &m.res_ident.as_ref().unwrap();
                    query.push_str(format!("  {}(): {}!\n", &m.method_ident, &res_ident).as_str());
                    let token = quote! {
                        #res_ident::schema(&mut register);
                    };
                    tokens = quote! {
                        #tokens
                        #token
                    };
                } else {
                    let payload_ident = &m.payload_ident.as_ref().unwrap();
                    let res_ident = &m.res_ident.as_ref().unwrap();
                    query.push_str(
                        format!(
                            "  {}(\n    payload: {}!\n  ): {}!\n",
                            &m.method_ident, &payload_ident, &res_ident
                        )
                        .as_str(),
                    );
                    let token = quote! {
                        #payload_ident::schema(&mut register);
                        #res_ident::schema(&mut register);
                    };
                    tokens = quote! {
                        #tokens
                        #token
                    };
                }
            }
        } else {
            if m.res_ident.is_none() {
                if m.payload_ident.is_none() {
                    mutation.push_str(format!("  {}()\n", &m.method_ident).as_str());
                } else {
                    let payload_ident = m.payload_ident.as_ref().unwrap();
                    mutation.push_str(
                        format!(
                            "  {}(\n    payload: {}!\n  )\n",
                            &m.method_ident, &payload_ident
                        )
                        .as_str(),
                    );
                    let token = quote! {
                        #payload_ident::schema(&mut register);
                    };
                    tokens = quote! {
                        #tokens
                        #token
                    };
                }
            } else {
                if m.payload_ident.is_none() {
                    let res_ident = &m.res_ident.as_ref().unwrap();
                    mutation
                        .push_str(format!("  {}(): {}!\n", &m.method_ident, &res_ident).as_str());
                    let token = quote! {
                        #res_ident::schema(&mut register);
                    };
                    tokens = quote! {
                        #tokens
                        #token
                    };
                } else {
                    let payload_ident = &m.payload_ident.as_ref().unwrap();
                    let res_ident = &m.res_ident.as_ref().unwrap();
                    mutation.push_str(
                        format!(
                            "  {}(\n    payload: {}!\n  ): {}!\n",
                            &m.method_ident, &payload_ident, &res_ident
                        )
                        .as_str(),
                    );
                    let token = quote! {
                        #payload_ident::schema(&mut register);
                        #res_ident::schema(&mut register);
                    };
                    tokens = quote! {
                        #tokens
                        #token
                    };
                }
            }
        }
    }

    if format!("type {}Mutation {}\n", service, "{") == mutation {
        mutation = "".to_owned();
    } else {
        mutation = mutation + "}\n\n";
    }
    if format!("type {}Query {}\n", service, "{") == query {
        query = "".to_owned();
    } else {
        query = query + "}\n\n";
    }

    let method = mutation + query.as_str();
    let method_token = quote! {
        let mut objects = "".to_owned();

        for schema in register.values() {
            objects.push_str(schema.as_str());
            objects.push_str("\n\n");
        }

        #method.to_owned() + objects.as_str()
    };
    quote! {
        #tokens
        #method_token
    }
}

pub fn gen_service_code(_: TokenStream, item: TokenStream) -> TokenStream {
    let impl_item = parse_macro_input!(item as ItemImpl);

    let service_ident = get_service_ident(&impl_item);
    let items = &impl_item.items;
    let (impl_generics, ty_generics, where_clause) = impl_item.generics.split_for_impl();

    let mut methods: Vec<ServiceMethod> = vec![];

    for item in items {
        if let ImplItem::Method(method) = item {
            if let Some(service_method) = find_service_method(method) {
                methods.push(service_method)
            }
        }
    }

    let genesis_method = find_genesis(items);
    let genesis_body = match genesis_method {
        Some(genesis_method) => get_genesis_body(&genesis_method),
        None => quote! {()},
    };

    let hooks = extract_hooks(items);
    let hook_before = &hooks.before;
    let hook_before_body = match hook_before {
        Some(hook_before) => quote! { self.#hook_before(_params) },
        None => quote! {()},
    };
    let hook_after = &hooks.after;
    let hook_after_body = match hook_after {
        Some(hook_after) => quote! { self.#hook_after(_params) },
        None => quote! {()},
    };
    let tx_hook_before = &hooks.tx_before;
    let tx_hook_before_body = match tx_hook_before {
        Some(tx_hook_before) => quote! { self.#tx_hook_before(_ctx) },
        None => quote! {()},
    };
    let tx_hook_after = &hooks.tx_after;
    let tx_hook_after_body = match tx_hook_after {
        Some(tx_hook_after) => quote! { self.#tx_hook_after(_ctx) },
        None => quote! {()},
    };

    let list_method_meta: Vec<MethodMeta> = methods.into_iter().map(extract_method_meta).collect();

    let schema_code = gen_schema_code(&service_ident, &list_method_meta);

    let (list_read_name, list_read_ident, list_read_payload) =
        split_list_for_metadata(&list_method_meta, true);
    let (list_write_name, list_write_ident, list_write_payload) =
        split_list_for_metadata(&list_method_meta, false);

    let (list_read_name_nonepayload, list_read_ident_nonepayload) =
        split_list_for_metadata_nonepayload(&list_method_meta, true);
    let (list_write_name_nonepayload, list_write_ident_nonepayload) =
        split_list_for_metadata_nonepayload(&list_method_meta, false);

    TokenStream::from(quote! {
        impl #impl_generics protocol::traits::Service for #service_ident #ty_generics #where_clause {
            fn schema_(&self) -> String {
                #schema_code
            }
            fn genesis_(&mut self, _payload: String) {
                #genesis_body
            }

            fn hook_before_(&mut self, _params: &ExecutorParams) {
                #hook_before_body
            }

            fn hook_after_(&mut self, _params: &ExecutorParams) {
                #hook_after_body
            }

            fn tx_hook_before_(&mut self, _ctx: ServiceContext) {
                #tx_hook_before_body
            }

            fn tx_hook_after_(&mut self, _ctx: ServiceContext) {
                #tx_hook_after_body
            }

            fn read_(&self, ctx: protocol::types::ServiceContext) -> ServiceResponse<String> {
                let service = ctx.get_service_name();
                let method = ctx.get_service_method();

                if method == "get_schema" {
                    return ServiceResponse::<String>::from_succeed(self.schema_());
                }

                match method {
                    #(#list_read_name => {
                        let payload_res: Result<#list_read_payload, _> = serde_json::from_str(ctx.get_payload());
                        if payload_res.is_err() {
                            return ServiceResponse::<String>::from_error(1, "decode service payload failed".to_owned());
                        };
                        let payload = payload_res.unwrap();
                        let res = self.#list_read_ident(ctx, payload);
                        if !res.is_error() {
                            let mut data_json = serde_json::to_string(&res.succeed_data).unwrap_or_else(|e| panic!("encode succeed_data of ServiceResponse failed: {:?}", e));
                            if data_json == "null" {
                                data_json = "".to_owned();
                            }
                            ServiceResponse::<String>::from_succeed(data_json)
                        } else {
                            ServiceResponse::<String>::from_error(res.code, res.error_message.clone())
                        }
                    },)*
                    #(#list_read_name_nonepayload => {
                        let res = self.#list_read_ident_nonepayload(ctx);
                        if !res.is_error() {
                            let mut data_json = serde_json::to_string(&res.succeed_data).unwrap_or_else(|e| panic!("encode succeed_data of ServiceResponse failed: {:?}", e));
                            if data_json == "null" {
                                data_json = "".to_owned();
                            }
                            ServiceResponse::<String>::from_succeed(data_json)
                        } else {
                            ServiceResponse::<String>::from_error(res.code, res.error_message.clone())
                        }
                    },)*
                    _ => ServiceResponse::<String>::from_error(2, format!("not found method:{:?} of service:{:?}", method, service))
                }
            }

            fn write_(&mut self, ctx: protocol::types::ServiceContext) -> ServiceResponse<String> {
                let service = ctx.get_service_name();
                let method = ctx.get_service_method();

                match method {
                    #(#list_write_name => {
                        let payload_res: Result<#list_write_payload, _> = serde_json::from_str(ctx.get_payload());
                        if payload_res.is_err() {
                            return ServiceResponse::<String>::from_error(1, "decode service payload failed".to_owned());
                        };
                        let payload = payload_res.unwrap();
                        let res = self.#list_write_ident(ctx, payload);
                        if !res.is_error() {
                            let mut data_json = serde_json::to_string(&res.succeed_data).unwrap_or_else(|e| panic!("encode succeed_data of ServiceResponse failed: {:?}", e));
                            if data_json == "null" {
                                data_json = "".to_owned();
                            }
                            ServiceResponse::<String>::from_succeed(data_json)
                        } else {
                            ServiceResponse::<String>::from_error(res.code, res.error_message.clone())
                        }
                    },)*
                    #(#list_write_name_nonepayload => {
                        let res = self.#list_write_ident_nonepayload(ctx);
                        if !res.is_error() {
                            let mut data_json = serde_json::to_string(&res.succeed_data).unwrap_or_else(|e| panic!("encode succeed_data of ServiceResponse failed: {:?}", e));
                            if data_json == "null" {
                                data_json = "".to_owned();
                            }
                            ServiceResponse::<String>::from_succeed(data_json)
                        } else {
                            ServiceResponse::<String>::from_error(res.code, res.error_message.clone())
                        }
                    },)*
                    _ => ServiceResponse::<String>::from_error(2, format!("not found method:{:?} of service:{:?}", method, service))
                }
            }
        }

        #impl_item
    })
}

fn split_list_for_metadata(
    list: &[MethodMeta],
    readonly: bool,
) -> (Vec<String>, Vec<Ident>, Vec<Ident>) {
    let mut methods = vec![];
    let mut method_idents = vec![];
    let mut payload_idents = vec![];

    list.iter()
        .filter(|meta| meta.readonly == readonly && meta.payload_ident.is_some())
        .for_each(|meta| {
            methods.push(meta.method_ident.to_string());
            method_idents.push(meta.method_ident.clone());
            payload_idents.push(
                meta.payload_ident
                    .as_ref()
                    .expect("MethodMeta should have payload ident")
                    .clone(),
            );
        });
    (methods, method_idents, payload_idents)
}

fn split_list_for_metadata_nonepayload(
    list: &[MethodMeta],
    readonly: bool,
) -> (Vec<String>, Vec<Ident>) {
    let mut methods = vec![];
    let mut method_idents = vec![];

    list.iter()
        .filter(|meta| meta.readonly == readonly && meta.payload_ident.is_none())
        .for_each(|meta| {
            methods.push(meta.method_ident.to_string());
            method_idents.push(meta.method_ident.clone());
        });
    (methods, method_idents)
}

fn get_service_ident(impl_item: &ItemImpl) -> Ident {
    match &*impl_item.self_ty {
        Type::Path(type_path) => type_path.path.segments[0].ident.clone(),
        _ => panic!("The identity of the service was not found."),
    }
}

fn find_service_method(method: &ImplItemMethod) -> Option<ServiceMethod> {
    let attrs = &method.attrs;

    for attr in attrs {
        for segment in &attr.path.segments {
            if segment.ident == READ_ATTRIBUTE {
                return Some(ServiceMethod::Read(method.clone()));
            } else if segment.ident == WRITE_ATTRIBUTE {
                return Some(ServiceMethod::Write(method.clone()));
            }
        }
    }

    None
}

fn find_genesis(items: &[ImplItem]) -> Option<ImplItemMethod> {
    let methods: Vec<ImplItemMethod> = find_list_for_item_method(items);

    let mut count = 0;
    let mut genesis: Option<ImplItemMethod> = None;

    for method in methods {
        for attr in &method.attrs {
            for segment in &attr.path.segments {
                if segment.ident == GENESIS_ATTRIBUTE {
                    if count == 0 {
                        genesis = Some(method.clone());
                        count = 1;
                    } else {
                        panic!("The genesis method can only have one")
                    }
                }
            }
        }
    }

    genesis
}

fn get_genesis_body(item: &ImplItemMethod) -> proc_macro2::TokenStream {
    let method_name = item.sig.ident.clone();
    match item.sig.inputs.len() {
        1 => quote!{ self.#method_name()},
        2 => {
                let payload_arg = &item.sig.inputs[1];
                let pat_type = match payload_arg {
                    FnArg::Typed(pat_type) => pat_type,
                    _ => unreachable!(),
                };

                let payload_ident = if let Type::Path(path) = &*pat_type.ty {
                    Some(path.path.get_ident().expect("No payload type found.").clone())
                } else {
                    panic!("No payload type found.")
                };

                quote!{
                    let payload: #payload_ident = serde_json::from_str(&_payload)
                    .unwrap_or_else(|e| panic!("decode genesis payload failed: {:?}", e));
                    self.#method_name(payload)
                }
        },
        _ => panic!("genesis method input params should be `(&mut self)` or `(&mut self, payload: PayloadType)`")
    }
}

fn extract_hooks(items: &[ImplItem]) -> Hooks {
    let methods: Vec<ImplItemMethod> = find_list_for_item_method(items);

    let mut hooks = Hooks {
        before:    None,
        after:     None,
        tx_before: None,
        tx_after:  None,
    };

    let mut before_count = 0;
    let mut after_count = 0;
    let mut tx_before_count = 0;
    let mut tx_after_count = 0;

    for method in methods {
        for attr in &method.attrs {
            for segment in &attr.path.segments {
                if segment.ident == HOOK_BEFORE_ATTRIBUTE {
                    if before_count == 0 {
                        hooks.before = Some(method.sig.ident.clone());
                        before_count = 1;
                    } else {
                        panic!("The before hook can only have one")
                    }
                } else if segment.ident == HOOK_AFTER_ATTRIBUTE {
                    if after_count == 0 {
                        hooks.after = Some(method.sig.ident.clone());
                        after_count = 1;
                    } else {
                        panic!("The after hook can only have one")
                    }
                } else if segment.ident == TX_HOOK_BEFORE_ATTRIBUTE {
                    if tx_before_count == 0 {
                        hooks.tx_before = Some(method.sig.ident.clone());
                        tx_before_count = 1;
                    } else {
                        panic!("The tx before hook can only have one")
                    }
                } else if segment.ident == TX_HOOK_AFTER_ATTRIBUTE {
                    if tx_after_count == 0 {
                        hooks.tx_after = Some(method.sig.ident.clone());
                        tx_after_count = 1;
                    } else {
                        panic!("The tx after hook can only have one")
                    }
                }
            }
        }
    }

    hooks
}

fn find_list_for_item_method(items: &[ImplItem]) -> Vec<ImplItemMethod> {
    items
        .iter()
        .filter(|item| {
            if let ImplItem::Method(_) = item {
                true
            } else {
                false
            }
        })
        .map(|item| {
            if let ImplItem::Method(method) = item {
                method.clone()
            } else {
                unreachable!()
            }
        })
        .collect()
}

fn extract_res_ident(output: &ReturnType) -> Option<Ident> {
    match output {
        ReturnType::Type(_, ty) => match &**ty {
            Type::Path(ty_path) => {
                let arg = &ty_path
                    .path
                    .segments
                    .first()
                    .expect("path should contain type")
                    .arguments;
                match &arg {
                    PathArguments::AngleBracketed(angle_arg) => {
                        let arg_enum = angle_arg.args.first().expect("path should contain type");
                        if let GenericArgument::Type(arg_ty) = arg_enum {
                            match arg_ty {
                                Type::Path(ret_ty) => Some(
                                    ret_ty
                                        .path
                                        .segments
                                        .first()
                                        .expect("ServiceResponse<T> should contain T")
                                        .ident
                                        .clone(),
                                ),
                                Type::Tuple(_) => None,
                                _ => panic!("arg should contain return type"),
                            }
                        } else {
                            panic!("ServiceResponse<T> should contain a type")
                        }
                    }
                    _ => panic!("return type should be AngleBracketed"),
                }
            }
            _ => panic!("return type of read/write method should be ServiceResponse<T>"),
        },
        _ => panic!("return type of read/write method should be ServiceResponse<T>"),
    }
}

fn extract_method_meta(method: ServiceMethod) -> MethodMeta {
    let (impl_method, readonly) = match method {
        ServiceMethod::Read(impl_method) => (impl_method, true),
        ServiceMethod::Write(impl_method) => (impl_method, false),
    };

    match &impl_method.sig.inputs.len() {
        // Method input params: `(&self/&mut self, ctx: ServiceContext)`
        2 => {
            let res_ident = extract_res_ident(&impl_method.sig.output);
            MethodMeta {
                method_ident: impl_method.sig.ident,
                payload_ident: None,
                readonly,
                res_ident
            }
        },
        // Method input params: `(&self/&mut self, ctx: ServiceContext, payload: PayloadType)`
        3 => {
            let payload_arg = &impl_method.sig.inputs[2];
            let pat_type = match payload_arg {
                FnArg::Typed(pat_type) => pat_type,
                _ => unreachable!(),
            };

            let payload_ident = if let Type::Path(path) = &*pat_type.ty {
                Some(path.path.get_ident().expect("No payload type found.").clone())
            } else {
                panic!("No payload type found.")
            };

            let res_ident = extract_res_ident(&impl_method.sig.output);

            MethodMeta {
                method_ident: impl_method.sig.ident,
                payload_ident,
                readonly,
                res_ident
            }
        },
        _ => panic!("Method input params should be `(&self/&mut self, ctx: ServiceContext)` or `(&self/&mut self, ctx: ServiceContext, payload: PayloadType)`")
    }
}
