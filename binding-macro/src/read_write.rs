use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, FnArg, ImplItemMethod, ReturnType, Token, Type, Visibility, PathArguments, GenericArgument
};

use crate::common::get_protocol_result_args;

pub fn verify_read_or_write(item: TokenStream, mutable: bool) -> TokenStream {
    let method_item = parse_macro_input!(item as ImplItemMethod);

    let visibility = &method_item.vis;
    let inputs = &method_item.sig.inputs;
    let ret_type = &method_item.sig.output;

    verify_visibiity(visibility);

    verify_inputs(inputs, mutable);

    verify_ret_type(ret_type);

    TokenStream::from(quote! {#method_item})
}

fn verify_visibiity(visibility: &Visibility) {
    match visibility {
        Visibility::Inherited => {}
        _ => panic!("The visibility of read/write method must be private"),
    };
}

fn verify_inputs(inputs: &Punctuated<FnArg, Token![,]>, mutable: bool) {
    if inputs.len() < 2 {
        panic!("The two required parameters are missing: `&self/&mut self` and `ServiceContext`.")
    }

    if mutable {
        if !arg_is_mutable_receiver(&inputs[0]) {
            panic!("The receiver must be `&mut self`.")
        }
    } else if !arg_is_inmutable_receiver(&inputs[0]) {
        panic!("The receiver must be `&self`.")
    }

    match &inputs[1] {
        FnArg::Typed(pt) => {
            let ty = pt.ty.as_ref();
            assert_ty_servicecontext(ty)
        },
        _ => panic!("The second parameter should be `ServiceContext`.")
    }
}

fn assert_ty_servicecontext(ty: &Type) {
    match ty {
        Type::Path(ty_path) => {
            let path = &ty_path.path;
            assert_eq!(path.leading_colon.is_none(), true);
            assert_eq!(path.segments.len(), 1);
            assert_eq!(path.segments[0].ident, "ServiceContext")
        },
        _ => panic!("The type should be `ServiceContext")
    }
}

fn verify_ret_type(ret_type: &ReturnType) {
    let real_ret_type = match ret_type {
        ReturnType::Type(_, t) => t.as_ref(),
        _ => panic!("The return type of read/write method must be protocol::ProtocolResult"),
    };

    match real_ret_type {
        Type::Path(type_path) => {
            let path = &type_path.path;
            let result_args = get_protocol_result_args(&path)
                .expect("The return type of read/write method must be protocol::ProtocolResult");

            match result_args {
                PathArguments::AngleBracketed(angle_args) => {
                    let generic_args = &angle_args.args[0];
                    match generic_args {
                        GenericArgument::Type(generic_type) => {
                            assert_type_impl_codec(&generic_type)
                        },
                        _ => panic!("ProtocolResult should contain a Type")
                    }
                },
                _ => panic!("The return type of read/write method must be protocol::ProtocolResult<T> or protocol::ProtocolResult<()>")
            }
        }
        _ => panic!("The return type of read/write method must be protocol::ProtocolResult"),
    }
}

fn assert_type_impl_codec(ty: &Type) {
    match ty {
        Type::Tuple(t) => {

        },
        Type::Path(p) => {
            let path = &p.path;
            assert_eq!(path.leading_colon.is_none(), true);
            // println!("debug: T is {:?}", path.segments[0].ident)
            assert_impl_all!(path.segments[0].ident.span(): Send);
        },
        _ => panic!("The Type in ProtocolResult should be () or Generic Type")
    }
}

// expect &mut self
fn arg_is_mutable_receiver(fn_arg: &FnArg) -> bool {
    match fn_arg {
        FnArg::Receiver(receiver) => receiver.reference.is_some() && receiver.mutability.is_some(),
        _ => false,
    }
}

// expect &self
fn arg_is_inmutable_receiver(fn_arg: &FnArg) -> bool {
    match fn_arg {
        FnArg::Receiver(receiver) => receiver.reference.is_some() && receiver.mutability.is_none(),
        _ => false,
    }
}
