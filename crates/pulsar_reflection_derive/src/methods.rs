//! `#[reflect_methods]`: register an impl block's `#[reflect_method]`
//! methods in `pulsar_reflection::methods`.
//!
//! For each marked method this generates an `invoke` shim (an associated
//! fn, so `Self` resolves naturally) and a `ReflectedMethod` entry; the
//! block's entries are one associated const, submitted to `inventory` as a
//! `TypeMethodRegistration`. Signature handling lives in
//! `pulsar_reflection_codegen`, shared with other crates' method macros.

use proc_macro2::{Span, TokenStream};
use pulsar_reflection_codegen::{take_marked, type_ref, MethodSpec, SelfKind};
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemImpl};

pub fn reflect_methods(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if !attr.is_empty() {
        let err = syn::Error::new(Span::call_site(), "#[reflect_methods] takes no arguments");
        return err.to_compile_error().into();
    }
    let item = parse_macro_input!(item as ItemImpl);
    match expand(item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand(mut item: ItemImpl) -> syn::Result<TokenStream> {
    if let Some((_, path, _)) = &item.trait_ {
        return Err(syn::Error::new_spanned(
            path,
            "#[reflect_methods] goes on an inherent impl block, not a trait impl",
        ));
    }
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "#[reflect_methods] needs a concrete type: methods are keyed by TypeId",
        ));
    }

    let specs = take_marked(&mut item.items, "reflect_method")
        .into_iter()
        .map(|(func, marker)| MethodSpec::parse(func, &marker, 0))
        .collect::<syn::Result<Vec<_>>>()?;
    if specs.is_empty() {
        return Ok(quote!(#item));
    }

    let r = quote!(::pulsar_reflection);
    let self_ty = &item.self_ty;
    // Rust already rejects two inherent methods with the same name, so the
    // first method's name makes this block's const unique for the type.
    let const_name = format_ident!("__PULSAR_REFLECTED_METHODS_{}", specs[0].ident);
    let args = format_ident!("args");

    let shims = specs.iter().map(|spec| {
        let shim = format_ident!("__pulsar_reflect_invoke_{}", spec.ident);
        let ident = &spec.ident;
        let (receiver, leading) = match spec.self_kind {
            SelfKind::None => (quote!(let _ = receiver;), vec![]),
            SelfKind::Ref => (
                quote!(let this = receiver.downcast_ref::<Self>()?;),
                vec![quote!(this)],
            ),
            SelfKind::Mut => (
                quote!(let mut receiver = receiver; let this = receiver.downcast_mut::<Self>()?;),
                vec![quote!(this)],
            ),
        };
        let extract = spec.extract_args(&r, &args);
        let call = spec.call(&r, quote!(Self::#ident), &leading);
        quote! {
            #[doc(hidden)]
            fn #shim(
                receiver: #r::methods::Receiver<'_>,
                #args: &mut [::std::boxed::Box<dyn ::std::any::Any>],
            ) -> ::std::result::Result<
                ::std::option::Option<::std::boxed::Box<dyn ::std::any::Any>>,
                #r::methods::CallError,
            > {
                #receiver
                #extract
                #call
            }
        }
    });
    let entries = specs.iter().map(|spec| {
        let shim = format_ident!("__pulsar_reflect_invoke_{}", spec.ident);
        let info = spec.info(&r);
        let receiver = match spec.self_kind {
            SelfKind::None => quote!(#r::methods::ReceiverKind::None),
            SelfKind::Ref => quote!(#r::methods::ReceiverKind::Ref),
            SelfKind::Mut => quote!(#r::methods::ReceiverKind::Mut),
        };
        quote!(#r::methods::ReflectedMethod { info: #info, receiver: #receiver, invoke: Self::#shim })
    });
    let ty = type_ref(&r, self_ty);

    Ok(quote! {
        #item

        #[doc(hidden)]
        #[allow(non_snake_case, non_upper_case_globals, clippy::needless_borrow, clippy::unit_arg)]
        impl #self_ty {
            #(#shims)*

            #[doc(hidden)]
            const #const_name: &'static [#r::methods::ReflectedMethod] = &[#(#entries),*];
        }

        #r::inventory::submit! {
            #r::methods::TypeMethodRegistration { ty: #ty, methods: <#self_ty>::#const_name }
        }
    })
}
