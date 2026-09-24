//! `#[reflect_methods]`: register an impl block's `#[reflect_method]`
//! methods in `pulsar_reflection::methods`.
//!
//! For each marked method this generates an `invoke` shim (an associated
//! fn, so `Self` resolves naturally) and a `ReflectedMethod` entry; the
//! block's entries are one associated const, submitted to `inventory` as a
//! `TypeMethodRegistration`.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Expr, ExprLit, FnArg, GenericArgument,
    ImplItem, ImplItemFn, ItemImpl, Lit, Meta, Pat, PathArguments, ReturnType, Type,
};

const MARKER: &str = "reflect_method";

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

    let mut methods = Vec::new();
    for impl_item in &mut item.items {
        let ImplItem::Fn(func) = impl_item else { continue };
        let Some(index) = func.attrs.iter().position(|a| a.path().is_ident(MARKER)) else {
            continue;
        };
        let marker = func.attrs.remove(index);
        methods.push(Method::parse(func, &marker)?);
    }

    if methods.is_empty() {
        return Ok(quote!(#item));
    }

    let self_ty = &item.self_ty;
    // Rust already rejects two inherent methods with the same name, so the
    // first method's name makes this block's const unique for the type.
    let const_name = format_ident!("__PULSAR_REFLECTED_METHODS_{}", methods[0].ident);
    let shims = methods.iter().map(Method::shim);
    let entries = methods.iter().map(Method::entry);

    Ok(quote! {
        #item

        #[doc(hidden)]
        #[allow(non_snake_case, non_upper_case_globals, clippy::needless_borrow, clippy::unit_arg)]
        impl #self_ty {
            #(#shims)*

            #[doc(hidden)]
            const #const_name: &'static [::pulsar_reflection::methods::ReflectedMethod] = &[
                #(#entries),*
            ];
        }

        ::pulsar_reflection::inventory::submit! {
            ::pulsar_reflection::methods::TypeMethodRegistration {
                ty: ::pulsar_reflection::methods::TypeRef {
                    id: ::std::any::TypeId::of::<#self_ty>,
                    name: ::std::any::type_name::<#self_ty>,
                },
                methods: <#self_ty>::#const_name,
            }
        }
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Value,
    Ref,
    Mut,
}

struct Param {
    name: String,
    /// The type stored in the argument slot. For `&str` this is `String`
    /// and for `&[T]` it is `Vec<T>`, since a slot must hold a sized value.
    slot_ty: Type,
    mode: Mode,
    /// `&str` / `&[T]`: pass the slot's contents as a slice.
    as_slice: bool,
}

struct Method {
    ident: syn::Ident,
    script_name: String,
    doc: String,
    receiver: Mode,
    has_receiver: bool,
    params: Vec<Param>,
    ret: Option<Type>,
    fallible: bool,
    side_effect_free: bool,
    deterministic: bool,
    attrs: Vec<(String, String)>,
}

impl Method {
    fn parse(func: &ImplItemFn, marker: &Attribute) -> syn::Result<Self> {
        let sig = &func.sig;
        if !sig.generics.params.is_empty() {
            return Err(syn::Error::new_spanned(
                &sig.generics,
                "reflected methods cannot be generic",
            ));
        }
        if let Some(asyncness) = &sig.asyncness {
            return Err(syn::Error::new_spanned(asyncness, "reflected methods cannot be async"));
        }

        let mut method = Method {
            ident: sig.ident.clone(),
            script_name: sig.ident.to_string(),
            doc: doc_string(&func.attrs),
            receiver: Mode::Value,
            has_receiver: false,
            params: Vec::new(),
            ret: None,
            fallible: false,
            side_effect_free: false,
            deterministic: false,
            attrs: Vec::new(),
        };
        method.parse_marker(marker)?;

        for (index, input) in sig.inputs.iter().enumerate() {
            match input {
                FnArg::Receiver(receiver) => {
                    if receiver.colon_token.is_some() || receiver.reference.is_none() {
                        return Err(syn::Error::new_spanned(
                            receiver,
                            "reflected methods take `&self` or `&mut self`",
                        ));
                    }
                    method.has_receiver = true;
                    method.receiver =
                        if receiver.mutability.is_some() { Mode::Mut } else { Mode::Ref };
                }
                FnArg::Typed(typed) => {
                    let name = match &*typed.pat {
                        Pat::Ident(ident) => ident.ident.to_string(),
                        _ => format!("arg{index}"),
                    };
                    method.params.push(Param::parse(name, &typed.ty)?);
                }
            }
        }

        if let ReturnType::Type(_, ty) = &sig.output {
            let (inner, fallible) = match result_ok_type(ty) {
                Some(ok) => (ok, true),
                None => ((**ty).clone(), false),
            };
            if matches!(inner, Type::Reference(_)) {
                return Err(syn::Error::new_spanned(
                    ty,
                    "reflected methods must return owned values",
                ));
            }
            method.fallible = fallible;
            method.ret = (!is_unit(&inner)).then_some(inner);
        }
        Ok(method)
    }

    /// `#[reflect_method]` or `#[reflect_method(pure, name = "x", key = "v")]`.
    fn parse_marker(&mut self, marker: &Attribute) -> syn::Result<()> {
        if matches!(marker.meta, Meta::Path(_)) {
            return Ok(());
        }
        marker.parse_nested_meta(|meta| {
            let key = meta
                .path
                .get_ident()
                .map(|i| i.to_string())
                .ok_or_else(|| meta.error("expected a flag or `key = \"value\"`"))?;
            if meta.input.peek(syn::Token![=]) {
                let value: syn::LitStr = meta.value()?.parse()?;
                if key == "name" {
                    self.script_name = value.value();
                } else {
                    self.attrs.push((key, value.value()));
                }
                return Ok(());
            }
            match key.as_str() {
                "pure" => {
                    self.side_effect_free = true;
                    self.deterministic = true;
                }
                "side_effect_free" => self.side_effect_free = true,
                "deterministic" => self.deterministic = true,
                _ => {
                    return Err(meta.error(
                        "unknown flag; expected `pure`, `side_effect_free`, `deterministic` \
                         or `key = \"value\"`",
                    ))
                }
            }
            Ok(())
        })
    }

    fn shim_ident(&self) -> syn::Ident {
        format_ident!("__pulsar_reflect_invoke_{}", self.ident)
    }

    fn shim(&self) -> TokenStream {
        let shim = self.shim_ident();
        let ident = &self.ident;
        let count = self.params.len();
        let p = quote!(::pulsar_reflection::methods::__private);

        let receiver = match (self.has_receiver, self.receiver) {
            (false, _) => quote!(let _ = receiver;),
            (true, Mode::Mut) => {
                quote!(let mut receiver = receiver; let this = receiver.downcast_mut::<Self>()?;)
            }
            (true, _) => quote!(let this = receiver.downcast_ref::<Self>()?;),
        };
        let checks = self.params.iter().enumerate().map(|(i, param)| {
            let ty = &param.slot_ty;
            quote!(#p::check_arg::<#ty>(args, #i)?;)
        });
        let names: Vec<_> = (0..count).map(|i| format_ident!("a{i}")).collect();
        let extracts = self.params.iter().zip(&names).map(|(param, name)| {
            let ty = &param.slot_ty;
            let value = match param.mode {
                Mode::Value => quote!(#p::take::<#ty>(slots.next().unwrap())),
                Mode::Ref => quote!(#p::borrow::<#ty>(slots.next().unwrap())),
                Mode::Mut => quote!(#p::borrow_mut::<#ty>(slots.next().unwrap())),
            };
            let value = if param.as_slice {
                match param.mode {
                    Mode::Mut => quote!(&mut #value[..]),
                    _ => quote!(&#value[..]),
                }
            } else {
                value
            };
            quote!(let #name = #value;)
        });
        let call = if self.has_receiver {
            quote!(Self::#ident(this, #(#names),*))
        } else {
            quote!(Self::#ident(#(#names),*))
        };
        let result = if self.fallible {
            quote!(#p::ret_result(#call))
        } else {
            quote!(::std::result::Result::Ok(#p::ret(#call)))
        };

        quote! {
            #[doc(hidden)]
            fn #shim(
                receiver: ::pulsar_reflection::methods::Receiver<'_>,
                args: &mut [::std::boxed::Box<dyn ::std::any::Any>],
            ) -> ::std::result::Result<
                ::std::option::Option<::std::boxed::Box<dyn ::std::any::Any>>,
                ::pulsar_reflection::methods::CallError,
            > {
                #receiver
                #p::check_arg_count(args, #count)?;
                #(#checks)*
                #[allow(unused_mut, unused_variables)]
                let mut slots = args.iter_mut();
                #(#extracts)*
                #result
            }
        }
    }

    fn entry(&self) -> TokenStream {
        let m = quote!(::pulsar_reflection::methods);
        let name = &self.script_name;
        let doc = &self.doc;
        let shim = self.shim_ident();
        let receiver = match (self.has_receiver, self.receiver) {
            (false, _) => quote!(#m::ReceiverKind::None),
            (true, Mode::Mut) => quote!(#m::ReceiverKind::Mut),
            (true, _) => quote!(#m::ReceiverKind::Ref),
        };
        let params = self.params.iter().map(|param| {
            let name = &param.name;
            let ty = type_ref(&param.slot_ty);
            let mode = match param.mode {
                Mode::Value => quote!(#m::PassMode::Value),
                Mode::Ref => quote!(#m::PassMode::Ref),
                Mode::Mut => quote!(#m::PassMode::Mut),
            };
            quote!(#m::ParamInfo { name: #name, ty: #ty, mode: #mode })
        });
        let ret = match &self.ret {
            Some(ty) => {
                let ty = type_ref(ty);
                quote!(::std::option::Option::Some(#ty))
            }
            None => quote!(::std::option::Option::None),
        };
        let side_effect_free = self.side_effect_free;
        let deterministic = self.deterministic;
        let attrs = self.attrs.iter().map(|(k, v)| quote!((#k, #v)));

        quote! {
            #m::ReflectedMethod {
                name: #name,
                doc: #doc,
                receiver: #receiver,
                params: &[#(#params),*],
                ret: #ret,
                flags: #m::MethodFlags {
                    side_effect_free: #side_effect_free,
                    deterministic: #deterministic,
                },
                attrs: &[#(#attrs),*],
                invoke: Self::#shim,
            }
        }
    }
}

impl Param {
    fn parse(name: String, ty: &Type) -> syn::Result<Self> {
        let Type::Reference(reference) = ty else {
            if let Type::ImplTrait(_) = ty {
                return Err(syn::Error::new_spanned(ty, "reflected methods cannot take `impl Trait`"));
            }
            return Ok(Self { name, slot_ty: ty.clone(), mode: Mode::Value, as_slice: false });
        };
        let mode = if reference.mutability.is_some() { Mode::Mut } else { Mode::Ref };
        let elem = &*reference.elem;
        let (slot_ty, as_slice) = match elem {
            Type::Path(path) if path.path.is_ident("str") => {
                if mode == Mode::Mut {
                    return Err(syn::Error::new_spanned(ty, "`&mut str` is not supported"));
                }
                (syn::parse_quote!(::std::string::String), true)
            }
            Type::Slice(slice) => {
                let inner = &slice.elem;
                (syn::parse_quote!(::std::vec::Vec<#inner>), true)
            }
            Type::Reference(_) => {
                return Err(syn::Error::new(ty.span(), "nested references are not supported"));
            }
            other => (other.clone(), false),
        };
        Ok(Self { name, slot_ty, mode, as_slice })
    }
}

fn type_ref(ty: &Type) -> TokenStream {
    quote! {
        ::pulsar_reflection::methods::TypeRef {
            id: ::std::any::TypeId::of::<#ty>,
            name: ::std::any::type_name::<#ty>,
        }
    }
}

fn is_unit(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

/// `T` for a return type spelled `Result<T, ..>` (any path ending in
/// `Result`, so aliases like `anyhow::Result<T>` count too).
fn result_ok_type(ty: &Type) -> Option<Type> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    if last.ident != "Result" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else { return None };
    match args.args.first()? {
        GenericArgument::Type(ok) => Some(ok.clone()),
        _ => None,
    }
}

fn doc_string(attrs: &[Attribute]) -> String {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(nv) => match &nv.value {
                Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Some(s.value()),
                _ => None,
            },
            _ => None,
        })
        .map(|line| line.strip_prefix(' ').unwrap_or(&line).to_string())
        .collect();
    lines.join("\n").trim().to_string()
}
