use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, FnArg, GenericArgument, ItemFn, LitStr, PathArguments, ReturnType, Type,
};

/// Implements `#[pulsar_conversion]` for one concrete, owned-value conversion.
pub fn pulsar_conversion(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);
    let options = parse_macro_input!(attr as ConversionOptions);
    match expand_conversion(options, item_fn) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

struct ConversionOptions {
    id: Option<LitStr>,
    label: Option<LitStr>,
}

impl syn::parse::Parse for ConversionOptions {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut options = Self {
            id: None,
            label: None,
        };
        let values =
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(input)?;
        for meta in values {
            let syn::Meta::NameValue(value) = meta else {
                return Err(syn::Error::new_spanned(
                    meta,
                    "expected `id = \"...\"` or `label = \"...\"`",
                ));
            };
            let syn::Expr::Lit(expr) = &value.value else {
                return Err(syn::Error::new_spanned(
                    value,
                    "conversion option must be a string literal",
                ));
            };
            let syn::Lit::Str(literal) = &expr.lit else {
                return Err(syn::Error::new_spanned(
                    expr,
                    "conversion option must be a string literal",
                ));
            };
            match value
                .path
                .get_ident()
                .map(|ident| ident.to_string())
                .as_deref()
            {
                Some("id") if options.id.is_none() => options.id = Some(literal.clone()),
                Some("label") if options.label.is_none() => options.label = Some(literal.clone()),
                Some("id" | "label") => {
                    return Err(syn::Error::new_spanned(
                        value,
                        "duplicate conversion option",
                    ));
                }
                _ => return Err(syn::Error::new_spanned(value, "unknown conversion option")),
            }
        }
        Ok(options)
    }
}

fn expand_conversion(
    options: ConversionOptions,
    item_fn: ItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    let signature = &item_fn.sig;
    if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &signature.generics,
            "reflected conversions must be monomorphic functions",
        ));
    }
    if signature.asyncness.is_some() || signature.unsafety.is_some() || signature.abi.is_some() {
        return Err(syn::Error::new_spanned(
            signature,
            "reflected conversions must be synchronous safe Rust functions",
        ));
    }
    if signature.variadic.is_some() {
        return Err(syn::Error::new_spanned(
            signature,
            "variadic conversions are not supported",
        ));
    }
    let [FnArg::Typed(argument)] = signature.inputs.iter().collect::<Vec<_>>().as_slice() else {
        return Err(syn::Error::new_spanned(
            &signature.inputs,
            "a reflected conversion must take exactly one owned value",
        ));
    };
    let source_ty = argument.ty.as_ref();
    if matches!(source_ty, Type::Reference(_)) {
        return Err(syn::Error::new_spanned(
            source_ty,
            "conversion source must be owned; references cannot be stored in the conversion registry",
        ));
    }
    let ReturnType::Type(_, return_ty) = &signature.output else {
        return Err(syn::Error::new_spanned(
            &signature.output,
            "a reflected conversion must return a target value",
        ));
    };

    let (target_ty, fallible) = result_target(return_ty)?;
    let function_name = &signature.ident;
    let source_id_fn = format_ident!("__pulsar_conversion_{}_source_id", function_name);
    let target_id_fn = format_ident!("__pulsar_conversion_{}_target_id", function_name);
    let source_name_fn = format_ident!("__pulsar_conversion_{}_source_name", function_name);
    let target_name_fn = format_ident!("__pulsar_conversion_{}_target_name", function_name);
    let convert_fn = format_ident!("__pulsar_conversion_{}_convert", function_name);
    let id = options
        .id
        .map(|s| quote!(#s))
        .unwrap_or_else(|| quote!(concat!(module_path!(), "::", stringify!(#function_name))));
    let default_label = if fallible { "FROM" } else { "INTO" };
    let label = options
        .label
        .unwrap_or_else(|| LitStr::new(default_label, function_name.span()));
    if label.value() != "INTO" && label.value() != "FROM" {
        return Err(syn::Error::new_spanned(
            label,
            "conversion label must be `INTO` or `FROM`",
        ));
    }

    let call = if fallible {
        quote! {
            let converted: #target_ty = #function_name(*value)
                .map_err(|error| ::pulsar_reflection::ReflectError::Custom(
                    ::std::string::ToString::to_string(&error)
                ))?;
            Ok(::std::boxed::Box::new(converted))
        }
    } else {
        quote! {
            let converted: #target_ty = #function_name(*value);
            Ok(::std::boxed::Box::new(converted))
        }
    };

    Ok(quote! {
        #item_fn

        #[doc(hidden)]
        fn #source_id_fn() -> ::std::any::TypeId {
            ::std::any::TypeId::of::<#source_ty>()
        }
        #[doc(hidden)]
        fn #target_id_fn() -> ::std::any::TypeId {
            ::std::any::TypeId::of::<#target_ty>()
        }
        #[doc(hidden)]
        fn #source_name_fn() -> &'static str {
            ::std::any::type_name::<#source_ty>()
        }
        #[doc(hidden)]
        fn #target_name_fn() -> &'static str {
            ::std::any::type_name::<#target_ty>()
        }
        #[doc(hidden)]
        fn #convert_fn(
            value: ::std::boxed::Box<dyn ::std::any::Any>,
        ) -> ::pulsar_reflection::ReflectResult<::std::boxed::Box<dyn ::std::any::Any>> {
            let value = value.downcast::<#source_ty>().map_err(|_| {
                ::pulsar_reflection::ReflectError::TypeMismatch {
                    expected: ::std::any::type_name::<#source_ty>(),
                    found: "different source value".to_string(),
                }
            })?;
            #call
        }

        ::pulsar_reflection::inventory::submit! {
            ::pulsar_reflection::ConversionRegistration {
                source_type_id: #source_id_fn,
                target_type_id: #target_id_fn,
                source_type_name: #source_name_fn,
                target_type_name: #target_name_fn,
                convert: #convert_fn,
                id: #id,
                label: #label,
            }
        }
    })
}

fn result_target(return_ty: &Type) -> syn::Result<(&Type, bool)> {
    let Type::Path(path) = return_ty else {
        return Ok((return_ty, false));
    };
    let Some(segment) = path.path.segments.last() else {
        return Ok((return_ty, false));
    };
    if segment.ident != "Result" {
        return Ok((return_ty, false));
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            return_ty,
            "expected `Result<Target, Error>`",
        ));
    };
    let mut types = arguments.args.iter().filter_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    let Some(target) = types.next() else {
        return Err(syn::Error::new_spanned(
            return_ty,
            "expected a target type in Result",
        ));
    };
    if types.next().is_none() {
        return Err(syn::Error::new_spanned(
            return_ty,
            "expected `Result<Target, Error>`",
        ));
    }
    Ok((target, true))
}
