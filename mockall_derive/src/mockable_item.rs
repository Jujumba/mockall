// vim: tw=80
use super::*;

/// An item that's ready to be mocked.
///
/// It should be functionally identical or near-identical to the original item,
/// but with minor alterations that make it suitable for mocking, such as
/// altered lifetimes.
pub(crate) enum MockableItem {
    Module(MockableModule),
    Struct(MockableStruct)
}

impl From<(Attrs, Item)> for MockableItem {
    fn from((attrs, item): (Attrs, Item)) -> MockableItem {
        match item {
            Item::Impl(item_impl) =>
                MockableItem::Struct(MockableStruct::from(item_impl)),
            Item::ForeignMod(item_foreign_mod) =>
                MockableItem::Module(
                    MockableModule::from((attrs, item_foreign_mod))
                ),
            Item::Mod(item_mod) =>
                MockableItem::Module(MockableModule::from(item_mod)),
            Item::Trait(trait_) =>
                MockableItem::Struct(MockableStruct::from((attrs, trait_))),
            _ => panic!("automock does not support this item type")
        }
    }
}

impl From<MockableStruct> for MockableItem {
    fn from(mock: MockableStruct) -> MockableItem {
        // TODO: stuff like deselfify
        MockableItem::Struct(mock)
    }
}

pub(crate) struct MockableModule {
    pub vis: Visibility,
    pub mock_ident: Ident,
    pub mod_token: token::Mod,
    /// Ident of the original module, if any
    pub orig_ident: Option<Ident>,
    pub content: Vec<Item>
}

impl From<(Attrs, ItemForeignMod)> for MockableModule {
    fn from((attrs, foreign): (Attrs, ItemForeignMod)) -> MockableModule {
        let orig_ident = None;
        let mock_ident = attrs.modname.expect(concat!(
            "module name is required when mocking foreign functions,",
            " like `#[automock(mod mock_ffi)]`"
        ));
        let mod_token = <Token![mod]>::default();
        let vis = Visibility::Public(VisPublic{
            pub_token: <Token![pub]>::default()
        });
        let content = foreign.items.into_iter()
            .map(|foreign_item| {
                match foreign_item {
                    ForeignItem::Fn(f) => {
                        let span = f.sig.span();
                        let mut sig = f.sig;
                        // Foreign functions are always unsafe.  Mock foreign
                        // functions should be unsafe too, to prevent "warning:
                        // unused unsafe" messages.
                        sig.unsafety = Some(Token![unsafe](span));
                        let block = Box::new(Block {
                            brace_token: token::Brace::default(),
                            stmts: Vec::new()
                        });
                        Item::Fn(
                            ItemFn {
                                attrs: f.attrs,
                                vis: f.vis,
                                sig,
                                block
                            }
                        )
                    },
                    _ => {
                        compile_error(foreign_item.span(),
                            "Unsupported foreign item type"
                        );
                        Item::Verbatim(TokenStream::default())
                    }
                }
            }).collect::<Vec<_>>();
        MockableModule { vis, mock_ident, mod_token, orig_ident, content }
    }
}

impl From<ItemMod> for MockableModule {
    fn from(mod_: ItemMod) -> MockableModule {
        let span = mod_.span();
        // TODO: in the future, consider mocking non-public modules
        let vis = Visibility::Public(VisPublic{
            pub_token: Token![pub](mod_.vis.span())
        });
        let mock_ident = format_ident!("mock_{}", mod_.ident);
        let orig_ident = Some(mod_.ident);
        let mod_token = mod_.mod_token;
        let content = if let Some((_, content)) = mod_.content {
            content
        } else {
            compile_error(span,
            "automock can only mock inline modules, not modules from another file");
            Vec::new()
        };
        // TODO: demutify funcs
        MockableModule { vis, mock_ident, mod_token, orig_ident, content }
    }
}
