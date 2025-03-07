use std::iter::Peekable;

use crate::{label::Label, map::{Trie, NodeRef}};
use louds_rs::LoudsNodeNum;
use crate::try_from::TryFromTokens;

/// Iterates through all the common prefixes of a given label.
#[derive(Clone)]
pub struct PrefixIter2<'t, Token, Value, Tokens: Iterator<Item = Token>> {
    trie: &'t Trie<Token, Value>,
    tokens: Peekable<Tokens>,
    curr: LoudsNodeNum,
    /// A temporary storage shared between iterations.
    children: Vec<LoudsNodeNum>,
}

impl<'t, Token, Value, Tokens: Iterator<Item = Token>> PrefixIter2<'t, Token, Value, Tokens> {
    #[inline]
    pub(crate) fn new<L: Label<Token, IntoTokens = Tokens>>(
        trie: &'t Trie<Token, Value>,
        label: L,
    ) -> Self {
        Self {
            trie,
            tokens: label.into_tokens().peekable(),
            curr: LoudsNodeNum(1),
            children: Vec::new(),
        }
    }

    pub fn labels<L>(self) -> impl Iterator<Item = L>
    where
        Token: Clone + Ord,
        L: TryFromTokens<Token> {
        self.filter_map(|node_ref| node_ref.label().ok())
    }

    /// TODO: The real version would return &Value not Value.
    pub fn pairs<L>(self) -> impl Iterator<Item = (L, Value)>
    where
        Token: Clone + Ord,
        L: TryFromTokens<Token>,
        Value: Clone, {
        self.filter_map(|node_ref| {
            // node_ref.range().pair.ok()
            let label = node_ref.label().ok();
            let value = node_ref.value().cloned();
            label.zip(value)
        })
    }

    /// TODO: The real version would return &Value not Value.
    pub fn values<L>(self) -> impl Iterator<Item = Value>
    where
        Token: Clone + Ord,
        L: TryFromTokens<Token>,
        Value: Clone{
        self.filter_map(|node_ref| {
            node_ref.value().cloned()
        })
    }

    // XXX: This doesn't make any sense here.
    // pub fn suffixes<L>(self) -> impl Iterator<Item = L>
    // where
    //     Token: Clone + Ord,
    //     L: TryFromTokens<Token> {
    //     let start: LoudsNodeNum = self.start;
    //     self.filter_map(move |node_ref| {
    //         L::try_from_reverse_tokens(node_ref.range_from(start).map(|n| n.token().clone())).ok()
    //     })
    // }
}

impl<'t, Token, Value, Tokens> Iterator for PrefixIter2<'t, Token, Value, Tokens>
where
    Token: Ord,
    Tokens: Iterator<Item = Token>,
{
    type Item = NodeRef<'t, Token, Value>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let token = self.tokens.next()?;

            self.children.clear();
            self.children
                .extend(self.trie.children_node_nums(self.curr));

            let idx = self
                .trie
                .bin_search_by_children_labels(&token, &self.children)
                .ok()?;
            self.curr = self.children[idx];

            if self.trie.value(self.curr).is_none() {
                continue;
            };

            return Some(NodeRef {
                trie: &self.trie,
                node_num: self.curr,
            });
        }
    }
}
