use std::iter::FromIterator;

use proc_macro_error::{abort, ResultExt};
use syn::{
    self, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Expr, ExprLit, Ident, Lit, LitStr, Token,
};

pub fn parse_clap_attributes(all_attrs: &[Attribute]) -> Vec<ClapAttr> {
    all_attrs
        .iter()
        .filter(|attr| attr.path.is_ident("clap") || attr.path.is_ident("structopt"))
        .flat_map(|attr| {
            attr.parse_args_with(Punctuated::<ClapAttr, Token![,]>::parse_terminated)
                .unwrap_or_abort()
        })
        .collect()
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum ClapAttr {
    // single-identifier attributes
    Short(Ident),
    Long(Ident),
    ValueParser(Ident),
    Action(Ident),
    Env(Ident),
    Flatten(Ident),
    ValueEnum(Ident),
    FromGlobal(Ident),
    Subcommand(Ident),
    VerbatimDocComment(Ident),
    ExternalSubcommand(Ident),
    About(Ident),
    Author(Ident),
    Version(Ident),

    // ident = "string literal"
    RenameAllEnv(Ident, LitStr),
    RenameAll(Ident, LitStr),
    NameLitStr(Ident, LitStr),

    // parse(parser_kind [= parser_func])
    Parse(Ident, ParserSpec),

    // ident [= arbitrary_expr]
    Skip(Ident, Option<Expr>),

    // ident = arbitrary_expr
    NameExpr(Ident, Expr),
    DefaultValueT(Ident, Option<Expr>),
    DefaultValueOsT(Ident, Option<Expr>),
    NextDisplayOrder(Ident, Expr),
    NextHelpHeading(Ident, Expr),
    HelpHeading(Ident, Expr),

    // ident(arbitrary_expr,*)
    MethodCall(Ident, Vec<Expr>),
}

impl Parse for ClapAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        use self::ClapAttr::*;

        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        if input.peek(Token![=]) {
            // `name = value` attributes.
            let assign_token = input.parse::<Token![=]>()?; // skip '='

            if input.peek(LitStr) {
                let lit: LitStr = input.parse()?;

                match &*name_str {
                    "rename_all" => Ok(RenameAll(name, lit)),
                    "rename_all_env" => Ok(RenameAllEnv(name, lit)),

                    "skip" => {
                        let expr = ExprLit {
                            attrs: vec![],
                            lit: Lit::Str(lit),
                        };
                        let expr = Expr::Lit(expr);
                        Ok(Skip(name, Some(expr)))
                    }

                    "next_display_order" => {
                        let expr = ExprLit {
                            attrs: vec![],
                            lit: Lit::Str(lit),
                        };
                        let expr = Expr::Lit(expr);
                        Ok(NextDisplayOrder(name, expr))
                    }

                    "next_help_heading" => {
                        let expr = ExprLit {
                            attrs: vec![],
                            lit: Lit::Str(lit),
                        };
                        let expr = Expr::Lit(expr);
                        Ok(NextHelpHeading(name, expr))
                    }
                    "help_heading" => {
                        let expr = ExprLit {
                            attrs: vec![],
                            lit: Lit::Str(lit),
                        };
                        let expr = Expr::Lit(expr);
                        Ok(HelpHeading(name, expr))
                    }

                    _ => Ok(NameLitStr(name, lit)),
                }
            } else {
                match input.parse::<Expr>() {
                    Ok(expr) => match &*name_str {
                        "skip" => Ok(Skip(name, Some(expr))),
                        "default_value_t" => Ok(DefaultValueT(name, Some(expr))),
                        "default_value_os_t" => Ok(DefaultValueOsT(name, Some(expr))),
                        "next_display_order" => Ok(NextDisplayOrder(name, expr)),
                        "next_help_heading" => Ok(NextHelpHeading(name, expr)),
                        "help_heading" => Ok(HelpHeading(name, expr)),
                        _ => Ok(NameExpr(name, expr)),
                    },

                    Err(_) => abort! {
                        assign_token,
                        "expected `string literal` or `expression` after `=`"
                    },
                }
            }
        } else if input.peek(syn::token::Paren) {
            // `name(...)` attributes.
            let nested;
            parenthesized!(nested in input);

            match name_str.as_ref() {
                "parse" => {
                    let parser_specs: Punctuated<ParserSpec, Token![,]> =
                        nested.parse_terminated(ParserSpec::parse)?;

                    if parser_specs.len() == 1 {
                        Ok(Parse(name, parser_specs[0].clone()))
                    } else {
                        abort!(name, "parse must have exactly one argument")
                    }
                }

                _ => {
                    let method_args: Punctuated<_, Token![,]> =
                        nested.parse_terminated(Expr::parse)?;
                    Ok(MethodCall(name, Vec::from_iter(method_args)))
                }
            }
        } else {
            // Attributes represented with a sole identifier.
            match name_str.as_ref() {
                "long" => Ok(Long(name)),
                "short" => Ok(Short(name)),
                "value_parser" => Ok(ValueParser(name)),
                "action" => Ok(Action(name)),
                "env" => Ok(Env(name)),
                "flatten" => Ok(Flatten(name)),
                "arg_enum" => Ok(ValueEnum(name)),
                "value_enum" => Ok(ValueEnum(name)),
                "from_global" => Ok(FromGlobal(name)),
                "subcommand" => Ok(Subcommand(name)),
                "external_subcommand" => Ok(ExternalSubcommand(name)),
                "verbatim_doc_comment" => Ok(VerbatimDocComment(name)),

                "default_value" => {
                    abort!(name,
                        "`#[clap(default_value)` attribute (without a value) has been replaced by `#[clap(default_value_t)]`.";
                        help = "Change the attribute to `#[clap(default_value_t)]`";
                    )
                }
                "default_value_t" => Ok(DefaultValueT(name, None)),
                "default_value_os_t" => Ok(DefaultValueOsT(name, None)),
                "about" => (Ok(About(name))),
                "author" => (Ok(Author(name))),
                "version" => Ok(Version(name)),

                "skip" => Ok(Skip(name, None)),

                _ => abort!(name, "unexpected attribute: {}", name_str),
            }
        }
    }
}

#[derive(Clone)]
pub struct ParserSpec {
    pub kind: Ident,
    pub eq_token: Option<Token![=]>,
    pub parse_func: Option<Expr>,
}

impl Parse for ParserSpec {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let kind = input
            .parse()
            .map_err(|_| input.error("parser specification must start with identifier"))?;
        let eq_token = input.parse()?;
        let parse_func = match eq_token {
            None => None,
            Some(_) => Some(input.parse()?),
        };
        Ok(ParserSpec {
            kind,
            eq_token,
            parse_func,
        })
    }
}
