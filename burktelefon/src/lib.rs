use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Expr, Lit};

#[proc_macro_derive(Burk, attributes(burk))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    // println!("{:#?}", &ast);
    let enum_name = ast.ident;
    let mut variants = Vec::new();
    let mut cmd_names = Vec::new();
    if let syn::Data::Enum(ref en) = ast.data {
        for variant in &en.variants {
            let ident = &variant.ident;
            let mut custom_name = None;
            for attr in &variant.attrs {
                if attr.path.is_ident("burk") {
                    let expr: Expr = attr.parse_args().unwrap();
                    // dbg!(&expr);
                    if let Expr::Assign(assign_expr) = expr {
                        if let Expr::Lit(exprlit) = *assign_expr.right {
                            if let Lit::Str(lit) = exprlit.lit {
                                custom_name = Some(lit.value());
                            };
                        } else {
                            panic!("no literal in right side of expression");
                        }
                    } else {
                        panic!("not assign expression in attribute");
                    }
                } else {
                    panic!("expected 'burk' found {:?}", attr.path)
                }
            }
            let cmd = custom_name.unwrap_or(ident.to_string().to_uppercase());
            cmd_names.push(cmd);
            variants.push(variant);
        }
    } else {
        panic!("'burk' can only be derived on enums!");
    }
    //let ident = variants[0].ident.to_string().to_uppercase();

    let mut match_code = quote! {};
    for (cmd_name, variant) in cmd_names.iter().zip(variants.iter()) {
        let ident = &variant.ident;
        let fields = &variant.fields;
        let mut construct_code = quote! {};
        for field in fields {
            let ty = &field.ty;
            construct_code.extend(quote! {
                parts.next().ok_or_else(|| make_error())?.parse::<#ty>().map_err(|_| make_error())?,
            });
        }
        if fields.is_empty() {
            match_code.extend(quote! {
                Some(#cmd_name) => {
                    core::result::Result::Ok(Self::#ident)
                },
            });
        } else {
            match_code.extend(quote! {
                Some(#cmd_name) => {
                    core::result::Result::Ok(Self::#ident (
                        #construct_code
                    ))
                },
            });
        }
    }

    let mut match_fmt = quote! {};
    for (cmd_name, variant) in cmd_names.iter().zip(variants.iter()) {
        let ident = &variant.ident;
        // let ident_upper = ident.to_string().to_uppercase();
        let unnamed_fields = if let syn::Fields::Unnamed(syn::FieldsUnnamed {
            unnamed: fields,
            ..
        }) = &variant.fields
        {
            fields.clone()
        } else {
            syn::punctuated::Punctuated::new()
        };
        let field_idents: Vec<_> = unnamed_fields
            .iter()
            .enumerate()
            .map(|(cnt, _)| {
                syn::Ident::new(&format!("f{cnt}"), proc_macro::Span::call_site().into())
            })
            .collect();
        let format_string: String = std::iter::repeat("{}")
            .take(field_idents.len() + 1)
            .collect::<Vec<_>>()
            .join(" ");

        if unnamed_fields.is_empty() {
            match_fmt.extend(quote! {
                Self::#ident => {
                    //write!(f, )
                    write!(f, #format_string, #cmd_name, #(#field_idents),*)
                }
            });
        } else {
            match_fmt.extend(quote! {
                Self::#ident (#(#field_idents),*) => {
                    //write!(f, )
                    write!(f, #format_string, #cmd_name, #(#field_idents),*)
                }
            });
        }
    }

    let code = quote! {
        impl core::str::FromStr for #enum_name {
            type Err = alloc::string::String;
            fn from_str(s: &str) -> Result<Self, Self::Err>  {
                let mut parts = s.split_whitespace();
                let make_error = || alloc::format!("failed to parse: {:?}", s);
                match parts.next() {
                    #match_code
                    _ => {Err(())},
                }.map_err(|_| make_error())
            }
        }
        impl core::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    #match_fmt
                }
            }
        }
    };
    code.into()
}
